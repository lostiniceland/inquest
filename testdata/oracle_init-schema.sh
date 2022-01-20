#!/usr/bin/bash

sql sys/Oracle18@oracle:1521/XEPDB1 as sysdba @oracle_init-schema.sql test changeit
sql test/changeit@@oracle:1521/XEPDB1 @testdata-oracle.sql