FROM phpmyadmin/phpmyadmin:5.2.2

ARG UNAGI_PASSWORD
RUN [ "${UNAGI_PASSWORD}" != '' ]
ENV UNAGI_PASSWORD=$UNAGI_PASSWORD

ENV PMA_HOST=/cloudsql/icfpc-primary:asia-northeast1:primary
ENV PMA_SOCKET=/cloudsql/icfpc-primary:asia-northeast1:primary
ENV PMA_USER=root
ENV PMA_PASSWORD=$UNAGI_PASSWORD

RUN { \
        echo 'AuthType Basic' && \
        echo 'AuthName "unagi:UNAGI_PASSWORD"' && \
        echo 'AuthUserFile /var/www/html/.htpasswd' && \
        echo 'Require valid-user'; \
    } > /var/www/html/.htaccess && \
    htpasswd -c -b /var/www/html/.htpasswd unagi "${UNAGI_PASSWORD}"
