# Pre-requisites

- Postgres-15
- Rust
- pgrx

# Getting Started

To run the program locally, clone the repository
`git clone https://github.com/tembo-io/clerk_fdw.git`

Run the program using the command
`cargo pgrx run`

Create the wrapper extension
`create extension clerk_fdw;`

Create the foreign data wrapper:

```
create foreign data wrapper clerk_wrapper
  handler clerk_fdw_handler
  validator clerk_fdw_validator;
```

Connect to clerk using your credentials:

```
create server my_clerk_server
  foreign data wrapper clerk_wrapper
  options (
    api_key '<clerk secret Key>')
```

Create Foreign Table:

## User table

```
create foreign table clerk_users (
  user_id text,
  first_name text,
  last_name text,
  email text,
  gender text,
  created_at bigint,
  updated_at bigint,
  last_sign_in_at bigint,
  phone_numbers bigint,
  username text
  )
  server my_clerk_server
  options (
      object 'users'
  );

```

## Organization Table

```
create foreign table clerk_orgs (
  organization_id text,
  name text,
  slug text,
  created_at bigint,
  updated_at bigint,
  created_by text
)
server my_clerk_server
options (
  object 'organizations'
);
```

Query from the Foreign Table:
`select * from clerk_users`

# SQL queries for most common tasks
