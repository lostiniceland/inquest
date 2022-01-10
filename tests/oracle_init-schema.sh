#!/usr/bin/bash

sql sys/Oracle18@localhost:1521/XEPDB1 as sysdba @oracle_init-schema.sql test changeit
sql test/changeit@@localhost:1521/XEPDB1 @testdata-oracle.sql