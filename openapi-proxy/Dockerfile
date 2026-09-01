FROM nginx:stable

RUN mkdir -p /run/ava && rm /etc/nginx/conf.d/default.conf

COPY nginx.conf /etc/nginx/nginx.conf
COPY ava.conf /etc/nginx/conf.d/ava.conf
COPY ava.js /etc/nginx/ava.js
