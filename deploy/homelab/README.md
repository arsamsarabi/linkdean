# Homelab deploy (nova + ncc)

Optional. App stays OSS-portable via env vars.

## nova

```bash
# on nova
mkdir -p ~/novabot/services/linkdean
# copy deploy/docker/docker-compose.yml + .env (from Infisical render or manual)
# publish host port e.g. 8128→8080
```

## ncc Traefik

File provider `services/traefik/config/traefik/conf/linkdean.yml`:

```yaml
http:
  routers:
    linkdean:
      rule: "Host(`linkdean.arsam.app`)"
      entryPoints: [websecure]
      tls:
        certResolver: letsencrypt
      service: linkdean
  services:
    linkdean:
      loadBalancer:
        servers:
          - url: "http://192.168.0.120:8128"
```

Public: add `linkdean.${DOMAIN}` to Cloudflare `DOMAINS`, recreate DDNS.

## Authentik

OIDC provider + Application launch `https://linkdean.arsam.app`.
Env on API:

```
OIDC_ISSUER=https://auth.arsam.app/application/o/linkdean/
OIDC_CLIENT_ID=...
OIDC_CLIENT_SECRET=...
APP_URL=https://linkdean.arsam.app
```

Bind group `household`. Callback: `https://linkdean.arsam.app/api/auth/oidc/callback`.
