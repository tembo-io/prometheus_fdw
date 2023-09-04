## Clerk_fdw

This section is about how to use the clerk foreign data wrapper in the data warehouse to collect valuable analytics

### Pre-requisistes

- have the v0.2.2 of `clerk_fdw` extension enabled in your instance

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

### User table

This table will store information about the users.
Note: The current limit is 500 users. This should be increased in future versions.

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

### Organization Table

This table will store information about the organizations.
Note: The current limit is 500 organizations. This should be increased in future versions.

```
create foreign table clerk_organizations (
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

### Junction Table

This table connects the `clerk_users` and `clerk_orgs`. It lists out all users and their roles in each organization.

```
create foreign table clerk_organization_memberships (
  user_id text,
  organization_id text,
  role text
)
server my_clerk_server
options (
  object 'organization_memberships'
);
```

NOTE: There is a 0.5 second sleep timer between each request so that we do not overload clerk servers. The reponse might take a while and it is reccomended that you store the information in a local table for quick access.

Query from the Foreign Table:
`select * from clerk_users`

To get all members of an organization:
`select * from organization_memberships where organization_id='org_id';`
