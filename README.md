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
`create extension clerk_fdw`

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
api_key '<clerk secret Key>' -- Clerk API key, required
);

```

Create Foreign Table:
```

create foreign table clerk (
id bigint,
name text,
email text
)
server my_clerk_server

```

Note: We will soon support being able to request more fields like orgranizations, roles etc.

Query from the Foreign Table:
`select * from clerk`
