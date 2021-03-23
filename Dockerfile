FROM nginx:latest

ADD target/wasm-dist/ /usr/share/nginx/html
