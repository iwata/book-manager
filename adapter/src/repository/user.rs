use async_trait::async_trait;
use derive_new::new;
use kernel::{
    model::{
        id::UserId,
        role::Role,
        user::{
            User,
            event::{CreateUser, DeleteUser, UpdateUserPassword, UpdateUserRole},
        },
    },
    repository::user::UserRepository,
};
use shared::error::{AppError, AppResult};

use crate::database::{ConnectionPool, model::user::UserRow};

#[derive(new)]
pub struct UserRepositoryImpl {
    db: ConnectionPool,
}

#[async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn find_current_user(&self, current_user_id: UserId) -> AppResult<Option<User>> {
        let row = sqlx::query_as!(
            UserRow,
            r#"
            select u.user_id, u.name, u.email, r.name as role_name, u.created_at, u.updated_at
            from users as u inner join roles as r using(role_id)
            where u.user_id = $1;
        "#,
            current_user_id as _
        )
        .fetch_optional(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        match row {
            Some(r) => Ok(Some(User::try_from(r)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> AppResult<Vec<User>> {
        let users = sqlx::query_as!(
            UserRow,
            r#"
            select u.user_id, u.name, u.email, r.name as role_name, u.created_at, u.updated_at
            from users as u inner join roles as r using(role_id)
            order by u.created_at desc;
        "#
        )
        .fetch_all(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?
        .into_iter()
        .filter_map(|row| User::try_from(row).ok())
        .collect();
        Ok(users)
    }

    async fn create(&self, event: CreateUser) -> AppResult<User> {
        let user_id = UserId::new();
        let hashed_password = hash_password(&event.password)?;
        let role = Role::User;

        let res = sqlx::query!(
            r#"
                insert into users(user_id, name, email, password_hash, role_id)
                select $1, $2, $3, $4, role_id from roles where name = $5;
            "#,
            user_id as _,
            event.name,
            event.email,
            hashed_password,
            role.as_ref()
        )
        .execute(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        if res.rows_affected() < 1 {
            return Err(AppError::NoRowsAffectedError(
                "No user has been updated".into(),
            ));
        }

        Ok(User {
            id: user_id,
            name: event.name,
            email: event.email,
            role: role,
        })
    }

    async fn update_password(&self, event: UpdateUserPassword) -> AppResult<()> {
        let mut tx = self.db.begin().await?;

        let original_password_hash = sqlx::query!(
            r#"
            select password_hash from users where user_id = $1;
        "#,
            event.user_id as _
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::SpecificOperationError)?
        .password_hash;

        verify_password(&event.current_password, &original_password_hash)?;

        let new_password_hash = hash_password(&event.new_password)?;
        sqlx::query!(
            r#"
             update users set password_hash = $2 where user_id = $1;
        "#,
            event.user_id as _,
            new_password_hash
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::SpecificOperationError)?;

        tx.commit().await.map_err(AppError::TransactionError)?;

        Ok(())
    }

    async fn update_role(&self, event: UpdateUserRole) -> AppResult<()> {
        let res = sqlx::query!(
            r#"
           update users set role_id = (
               select role_id from roles where name = $2
           ) where user_id = $1;
        "#,
            event.user_id as _,
            event.role.as_ref(),
        )
        .execute(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        if res.rows_affected() < 1 {
            return Err(AppError::EntityNotFound("Specified user not fount".into()));
        }

        Ok(())
    }

    async fn delete(&self, event: DeleteUser) -> AppResult<()> {
        let res = sqlx::query!(
            r#"delete from users where user_id = $1"#,
            event.user_id as _
        )
        .execute(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        if res.rows_affected() < 1 {
            return Err(AppError::EntityNotFound("Specified user not found".into()));
        }

        Ok(())
    }
}

fn hash_password(password: &str) -> AppResult<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(AppError::from)
}

fn verify_password(password: &str, hash: &str) -> AppResult<()> {
    let valid = bcrypt::verify(password, hash)?;
    if !valid {
        return Err(AppError::UnauthenticatedError);
    }
    Ok(())
}
