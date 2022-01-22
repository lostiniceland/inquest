CREATE TABLE testdata (
    some_smallserial smallserial,
    some_serial serial,
    some_bigserial bigserial,
    some_bit bit(3),
    some_boolean bool,
    some_varchar    varchar(10),
    some_char    char,
    some_text   text,
    some_name name,
    some_int int,
    some_int2 int2,
    some_int4 int4,
    some_int8 int8,
    some_smallint smallint,
    some_bigint bigint,
    some_float4 float4,
    some_float8 float8,
    some_real real,
    some_double double precision,
    some_timestamp timestamp,
    some_timestamptz timestamptz,
    some_macaddress macaddr,
    some_inet inet,
    some_time time,
    some_date date,
    some_geo_point point,
    some_geo_box box,
    some_geo_path path,
    some_json json
--  for now enough
);

INSERT INTO testdata (
    some_bit,
    some_boolean,
    some_varchar,
    some_char,
    some_text,
    some_name,
    some_int,
    some_int2,
    some_int4,
    some_int8,
    some_smallint,
    some_bigint,
    some_float4,
    some_float8,
    some_real,
    some_double,
    some_timestamp,
    some_timestamptz,
    some_macaddress,
    some_inet,
    some_time,
    some_date,
    some_geo_point,
    some_geo_box,
    some_geo_path,
    some_json
) VALUES (
    B'101',
    true,
    'varchar-10',
    'c',
    text('text'),
    name('name'),
    1,
    10,
    100,
    1000,
    1,
    10000,
    div(2,3),
    div(2,3),
    div(2,3),
    div(2,3),
    current_timestamp,
    current_timestamp,
    macaddr('08:00:2b:01:02:03'),
    inet_client_addr(),
    current_time,
    date(current_timestamp),
    point(2, 3),
    box(circle(point(1,1), 2.0)),
    path(
            '(15.878137629895164,47.08306448089695),
             (15.56169808311181,47.219041634920686),
             (15.267442604782124,47.4201665137259)'
        ),
    '{ "customer": "John Doe", "items": {"product": "Beer","qty": 6}}'
);

INSERT INTO testdata DEFAULT VALUES;