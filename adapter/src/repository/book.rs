use async_trait::async_trait;
use derive_new::new;
use kernel::{
    model::{
        book::{Book, event::CreateBook},
        id::BookId,
    },
    repository::book::BookRepository,
};
use shared::error::{AppError, AppResult};

use crate::database::{ConnectionPool, model::book::BookRow};

#[derive(new)]
pub struct BookRepositoryImpl {
    db: ConnectionPool,
}

#[async_trait]
impl BookRepository for BookRepositoryImpl {
    async fn create(&self, event: CreateBook) -> AppResult<()> {
        sqlx::query!(
            r#"
            insert into books(title,author,isbn,description)
            values($1,$2,$3,$4)
        "#,
            event.title,
            event.author,
            event.isbn,
            event.description,
        )
        .execute(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        Ok(())
    }

    async fn find_all(&self) -> AppResult<Vec<Book>> {
        let rows: Vec<BookRow> = sqlx::query_as!(
            BookRow,
            r#"
            select book_id,title,author,isbn,description
            from books
            order by created_at desc
        "#
        )
        .fetch_all(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        Ok(rows.into_iter().map(Book::from).collect())
    }

    async fn find_by_id(&self, book_id: BookId) -> AppResult<Option<Book>> {
        let row: Option<BookRow> = sqlx::query_as!(
            BookRow,
            r#"
            select book_id, title,author,isbn,description
            from books where book_id=$1
        "#,
            book_id as _
        )
        .fetch_optional(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        Ok(row.map(Book::from))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[sqlx::test]
    async fn test_register_book(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let repo = BookRepositoryImpl::new(ConnectionPool::new(pool));

        let book = CreateBook {
            title: "Test Title".into(),
            author: "Test author".into(),
            isbn: "Test ISBN".into(),
            description: "Test Description".into(),
        };
        repo.create(book).await?;

        let res = repo.find_all().await?;
        assert_eq!(res.len(), 1);

        let book_id = res[0].id;
        let res = repo.find_by_id(book_id.into()).await?;
        assert!(res.is_some());

        let Book {
            id,
            title,
            author,
            isbn,
            description,
        } = res.unwrap();
        assert_eq!(id, book_id);
        assert_eq!(title, "Test Title");
        assert_eq!(author, "Test author");
        assert_eq!(isbn, "Test ISBN");
        assert_eq!(description, "Test Description");

        Ok(())
    }
}
