# noverterm relay

Gives noverterm **Links** a public HTTPS URL: a Link named `myapp` on relay
`link.example.com` is served at `https://myapp.link.example.com`.

The relay is an SSH server. noverterm connects with its Link key and asks for a
remote forward (`ssh -R myapp:80:...`); visitors reach that tunnel through
Caddy, which gets a certificate per subdomain the first time it is visited.
Only keys in `authorized_keys` can connect, so the relay is yours alone.

## Deploy

You need a server with a public IP and a domain you control.

1. **DNS**: point both `link.example.com` and `*.link.example.com` (A records)
   at the server.
2. **Firewall**: open TCP 80, 443 and 2222.
3. **Configure**, on the server, from a clone of this repository:

   ```sh
   cd packages/relay/deploy
   cp .env.example .env        # set BASE_DOMAIN and ACME_EMAIL
   mkdir -p data
   ```

4. **Authorize noverterm**: in noverterm open **Link**, copy the Link key and
   append it to `data/authorized_keys` (one key per line). Edits apply to the
   next connection, no restart needed.
5. **Start**:

   ```sh
   docker compose up -d --build
   ```

6. In noverterm, set the relay to `link.example.com` and start a Link.

## Without noverterm

Any OpenSSH client with an authorized key works:

```sh
ssh -N -p 2222 -R myapp:80:localhost:3000 link@link.example.com
```

## Configuration

| Variable          | Default                | Meaning                                     |
| ----------------- | ---------------------- | ------------------------------------------- |
| `BASE_DOMAIN`     | (required)             | Tunnels are served at `<name>.BASE_DOMAIN`. |
| `AUTHORIZED_KEYS` | `/data/authorized_keys`| OpenSSH public keys allowed to connect.     |
| `HOST_KEY_PATH`   | `/data/host_key`       | Relay's SSH host key, created on first run. |
| `SSH_PORT`        | `2222`                 | SSH listener.                               |
| `INGRESS_PORT`    | `8080`                 | HTTP from Caddy.                            |
| `ASK_PORT`        | `8081`                 | Caddy's on-demand TLS check. Keep private.  |

Names are DNS labels (`a-z`, `0-9`, `-`), first come first served, released
when the tunnel disconnects. HTTP only (WebSockets included).
