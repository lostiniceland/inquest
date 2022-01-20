-- Param 1: user/schema
-- Param 2: passwort

BEGIN
  -- Create
  EXECUTE IMMEDIATE 'create user ' || '&1' || ' identified by ' || '"&2"' || ' default tablespace users temporary tablespace temp quota unlimited on users';
  EXECUTE IMMEDIATE 'grant connect to ' || '&1';
  EXECUTE IMMEDIATE 'grant create session to ' || '&1';
  EXECUTE IMMEDIATE 'grant resource to ' || '&1';
  EXECUTE IMMEDIATE 'grant unlimited tablespace to ' || '&1';
END;
/

exit;



