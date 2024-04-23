CREATE FUNCTION basic_setup(base_url text)
    RETURNS void
AS 'MODULE_PATHNAME', 'basic_setup'
    LANGUAGE C STRICT;

CREATE FUNCTION create_tables()
    RETURNS void
AS 'MODULE_PATHNAME', 'create_tables'
    LANGUAGE C STRICT;

CREATE FUNCTION create_indexes()
    RETURNS void
AS 'MODULE_PATHNAME', 'create_indexes'
    LANGUAGE C STRICT;

CREATE FUNCTION create_partitions(retention_period text)
    RETURNS void
AS 'MODULE_PATHNAME', 'create_partitions'
    LANGUAGE C STRICT;
