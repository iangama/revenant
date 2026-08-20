FROM node:20-alpine AS builder
WORKDIR /build
COPY web/control-panel/package.json web/control-panel/package-lock.json ./
RUN npm ci
COPY web/control-panel ./
RUN npm run build

FROM nginx:1.27-alpine
COPY infra/inspector.nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /build/dist /usr/share/nginx/html
EXPOSE 80
