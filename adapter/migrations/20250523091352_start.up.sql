-- Add up migration script here
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS '
  BEGIN
    new.updated_at := ''now'';
    return new;
  END;
' LANGUAGE 'plpgsql';

CREATE TABLE IF NOT EXISTS roles (
  role_id UUID primary key default gen_random_uuid(),
  name varchar(255) not null unique
);

CREATE TABLE IF NOT EXISTS users (
  user_id UUID primary key default gen_random_uuid(),
  name varchar(255) not null,
  email varchar(255) not null unique,
  password_hash varchar(255) not null,
  role_id uuid not null,
  created_at timestamp(3) with time zone not null default CURRENT_TIMESTAMP(3),
  updated_at timestamp(3) with time zone not null default CURRENT_TIMESTAMP(3),

  foreign key (role_id) references roles(role_id) on update cascade on delete cascade
);

create trigger users_updated_at_trigger
  before update on users for each row
  execute procedure set_updated_at();

CREATE TABLE IF NOT EXISTS books (
  book_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  title VARCHAR(255) NOT NULL,
  author VARCHAR(255) NOT NULL,
  isbn VARCHAR(255) NOT NULL,
  description VARCHAR(1024) NOT NULL,
  user_id UUID not null,
  created_at TIMESTAMP(3) WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at TIMESTAMP(3) WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

  FOREIGN KEY(user_id) REFERENCES users(user_id) on update cascade on delete cascade
);

CREATE TRIGGER books_updated_at_trigger
  BEFORE UPDATE ON books FOR EACH ROW
  EXECUTE PROCEDURE set_updated_at();

