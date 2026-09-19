FROM node:20-alpine AS build

WORKDIR /app

COPY frontend/package.json frontend/package-lock.json ./
COPY frontend/scripts ./scripts
RUN npm ci --ignore-scripts && node scripts/copy-ort-wasm.js

COPY frontend/tsconfig.json ./
COPY frontend/public ./public
COPY frontend/src ./src

ENV CI=true \
    GENERATE_SOURCEMAP=false \
    REACT_APP_API_URL=/api \
    REACT_APP_WS_URL=/ws
RUN npx react-scripts build

FROM nginxinc/nginx-unprivileged:1.27-alpine

COPY docker/web/nginx.conf /etc/nginx/conf.d/default.conf
COPY docker/web/entrypoint.sh /docker-entrypoint.d/40-rift-config.sh
COPY --from=build --chown=101:101 /app/build /usr/share/nginx/html

EXPOSE 8080
