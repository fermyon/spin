FROM mysql:8.0.35

ENV MYSQL_ROOT_PASSWORD=password
ENV MYSQL_DATABASE=spin_dev
ENV MYSQL_USER=spin
ENV MYSQL_PASSWORD=spin

HEALTHCHECK --start-period=10s --interval=2s --retries=30 CMD  /usr/bin/mysqladmin ping -h 127.0.0.1 -ppassword

EXPOSE 3306