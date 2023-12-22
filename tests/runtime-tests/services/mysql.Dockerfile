FROM mysql

ENV MYSQL_ROOT_PASSWORD=spin
ENV MYSQL_DATABASE=spin_dev
ENV MYSQL_USER=spin
ENV MYSQL_PASSWORD=spin

HEALTHCHECK --start-period=4s --interval=1s CMD  /usr/bin/mysqladmin ping --silent

EXPOSE 3306