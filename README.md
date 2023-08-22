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

```

create foreign table clerk (
  id text,
  first_name text,
  email text,
  last_name text,
  gender text,
  created_at bigint,
  last_sign_in_at bigint,
  phone_numbers bigint,
  username text,
  updated_at bigint,
  organization text,
  role text
  )
  server my_clerk_server;

```

This wrapper currently only supports displaying the name and email.
Note: We will soon support being able to request more fields like orgranizations, roles etc.

Query from the Foreign Table:
`select * from clerk`

# SQL queries for most common tasks

To display all organizations
`SELECT DISTINCT unnest(string_to_array(organization, ',')) AS unique_organization FROM clerk;`

To list all Users
`SELECT id, first_name, last_name, email FROM clerk;`

To list all Users of an Organization with Role

```
WITH org_roles AS (
  SELECT
    id,
    first_name,
    last_name,
    UNNEST(STRING_TO_ARRAY(organization, ',')) AS org,
    UNNEST(STRING_TO_ARRAY(role, ',')) AS org_role
  FROM clerk
)
SELECT
  id,
  first_name,
  last_name,
  org_role AS specific_role
FROM org_roles
WHERE org = 'OrgName';

```
