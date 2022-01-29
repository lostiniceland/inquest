FROM postgres

COPY --chown=postgres --chmod=400 certs/server.* /certs/
COPY --chown=postgres --chmod=400 certs/customCA.* /certs/
COPY postgres.conf /etc/postgresql/postgresql.conf
COPY postgres_hba.conf /etc/postgresql/pg_hba.conf
