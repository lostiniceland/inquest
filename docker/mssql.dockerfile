FROM mcr.microsoft.com/mssql/server

COPY --chmod=440 certs/server.* /certs/
COPY --chmod=440 certs/customCA.* /certs/
COPY --chown=mssql mssql.conf /var/opt/mssql/mssql.conf
#USER root
#RUN chmod 755 /certs
#USER mssql