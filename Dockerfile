FROM nginx:latest

COPY target/wasm-dist/* /usr/share/nginx/html/
