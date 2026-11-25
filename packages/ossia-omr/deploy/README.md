# Self-host Ossia OMR

Ossia OMR converts PDF and score images to MusicXML using Audiveris. This guide
runs the service on Ubuntu 24.04 x86-64 with systemd and connects it to Ossia.
The service processes one job at a time. Allow enough memory and disk for
Audiveris, uploaded scores and results; the example systemd unit limits the
service to 1300 MiB of memory and 2 GiB of swap.

## Build the service

From the root of an Ossia source checkout, build a Linux x86-64 binary with
Docker and extract it:

```sh
docker buildx build --platform linux/amd64 --load \
  -f packages/ossia-omr/deploy/Dockerfile -t ossia-omr-builder .
docker create --platform linux/amd64 --name ossia-omr-build ossia-omr-builder
docker cp ossia-omr-build:/src/target/release/ossia-omr ./ossia-omr-linux-amd64
docker rm ossia-omr-build
```

Alternatively, install Rust 1.91 or newer and the build dependencies `cmake`,
`clang`, `libclang-dev` and `pkg-config`, then run
`cargo build --release --locked -p ossia-omr`. Use a Linux x86-64 build for the
installation below. Copy the binary and `packages/ossia-omr/deploy/` to the
server if building on another machine. Run the remaining commands on the server
from the source checkout root (or adapt the paths to the copied files).

## Install Audiveris and language data

Install the [Audiveris 5.11.0 Ubuntu 24.04 package](https://github.com/Audiveris/audiveris/releases/tag/5.11.0)
from the upstream release. The installer includes its Java runtime. The XDG
directories let its desktop-menu setup succeed on a minimal server. Audiveris
requires the GTK 3 shared library even when running without a desktop.

```sh
sudo apt update
sudo apt install -y curl ca-certificates libgtk-3-dev
curl -fL -o /tmp/audiveris.deb \
  https://github.com/Audiveris/audiveris/releases/download/5.11.0/Audiveris-5.11.0-ubuntu24.04-x86_64.deb
sudo mkdir -p /usr/share/desktop-directories /usr/share/applications /etc/xdg/menus
sudo apt install /tmp/audiveris.deb
/opt/audiveris/bin/Audiveris -version
```

Ossia OMR requires an Audiveris heap limit of at most 1300 MiB. Set the
`-Xmx` option in `/opt/audiveris/lib/app/Audiveris.cfg` to `-Xmx1024m` (retain
any surrounding `java-options=` prefix). Check it again after upgrading or
reinstalling Audiveris:

```sh
sudoedit /opt/audiveris/lib/app/Audiveris.cfg
grep -- '-Xmx' /opt/audiveris/lib/app/Audiveris.cfg
```

Create the service account and data directories, then install the English OCR
language data from [Tesseract tessdata](https://github.com/tesseract-ocr/tessdata):

```sh
sudo useradd --system --user-group --home /var/lib/ossia-omr --shell /usr/sbin/nologin ossia-omr
sudo install -d -o ossia-omr -g ossia-omr -m 0755 /var/lib/ossia-omr/tessdata
curl -fL -o /tmp/eng.traineddata https://github.com/tesseract-ocr/tessdata/raw/main/eng.traineddata
sudo install -o ossia-omr -g ossia-omr -m 0644 /tmp/eng.traineddata /var/lib/ossia-omr/tessdata/eng.traineddata
```

For additional languages, download their `.traineddata` files from the same
repository into that directory and list their names in `OSSIA_OMR_LANGUAGES`,
separated by commas. Every configured file must be readable by the service.

## Configure and start

```sh
sudo install -o root -g root -m 0755 ossia-omr-linux-amd64 /usr/local/bin/ossia-omr
sudo install -o root -g root -m 0644 packages/ossia-omr/deploy/ossia-omr.service /etc/systemd/system/ossia-omr.service
sudo install -o root -g root -m 0600 packages/ossia-omr/deploy/ossia-omr.env.example /etc/ossia-omr.env
sudoedit /etc/ossia-omr.env
```

Set `OSSIA_OMR_TOKEN` to a long random single-line value without whitespace;
`openssl rand -hex 32` generates a suitable value. Set `OSSIA_OMR_LANGUAGES=eng`
for the language file installed above. Keep the environment file mode `0600`.
The default listen address is `127.0.0.1:3212`; choose another port if that
port is already in use, including by a local Ossia development backend.

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now ossia-omr
sudo systemctl status ossia-omr
sudo journalctl -u ossia-omr -n 100 --no-pager
```

The startup log lists installed OCR languages. If startup fails, check the
Audiveris version and heap setting, language file permissions, token and listen
address. Audiveris extracts native libraries into temporary directories; the
service's data filesystem must allow execution. Review logs with `journalctl`
when a job fails or exceeds its deadline.

Available settings:

| Variable                    | Purpose                                                                                               |
| --------------------------- | ----------------------------------------------------------------------------------------------------- |
| `OSSIA_OMR_HOME`            | Absolute data directory; default `/var/lib/ossia-omr`. The supplied systemd unit allows writes there. |
| `OSSIA_OMR_LISTEN`          | IP address and port; default `127.0.0.1:3212`.                                                        |
| `OSSIA_OMR_TOKEN`           | Required bearer token.                                                                                |
| `AUDIVERIS_BIN`             | Audiveris executable; default `/opt/audiveris/bin/Audiveris`.                                         |
| `OSSIA_OMR_LANGUAGES`       | Comma- or plus-separated OCR language names, matching files in `<home>/tessdata`.                     |
| `OSSIA_OMR_MAX_JOB_SECONDS` | Per-job deadline in seconds; default `1800`.                                                          |

If changing the data directory, update both the environment file and the
systemd unit's `ReadWritePaths` and directory ownership. Restart the service
after changing its configuration.

## Expose the service and connect Ossia

For access from another machine, put a TLS-terminating reverse proxy in front
of the loopback listener. Configure a certificate for your domain, forward
requests and the `Authorization` header, and allow at least 129 MiB per request
(the file limit is 128 MiB). For example, with TLS already configured in Nginx:

```nginx
server {
    listen 443 ssl;
    server_name omr.example.com;
    ssl_certificate /path/to/fullchain.pem;
    ssl_certificate_key /path/to/privkey.pem;
    client_max_body_size 130m;

    location / {
        proxy_pass http://127.0.0.1:3212;
        proxy_set_header Host $host;
        proxy_set_header Authorization $http_authorization;
    }
}
```

Replace the domain and certificate paths, validate the proxy configuration and
reload it. Protect the token and proxy logs, and expose only the TLS endpoint.

In Ossia, open **Settings → General → Score recognition service (OMR)**, select **Custom**,
enter `https://omr.example.com` and the same access key, and save. The URL must
not end with `/`. HTTP is accepted only for loopback addresses, so a locally
running service can use `http://127.0.0.1:3212`; a remote address requires
HTTPS. If the proxy uses a path prefix, include it in the service address and
configure the proxy to strip that prefix before forwarding to the service.

## Verify a recognition job

Install `curl` and `jq` on the machine running these commands. Set the URL and
access key to the same values configured in Ossia; the example tests a service
on the local machine. Substitute a real PDF score for the file path.

```sh
OMR_URL=http://127.0.0.1:3212
read -rs -p 'OMR access key: ' OMR_TOKEN; printf '\n'
curl -fsS "$OMR_URL/v1/health" | jq .
curl -fsS -H "Authorization: Bearer $OMR_TOKEN" "$OMR_URL/v1/capabilities" | jq .
JOB_ID=$(curl -fsS -H "Authorization: Bearer $OMR_TOKEN" \
  -F 'metadata={"sourceFormat":"pdf"};type=application/json' \
  -F 'file=@/absolute/path/to/score.pdf;type=application/pdf' \
  "$OMR_URL/v1/jobs" | jq -er .jobId)
printf 'Job: %s\n' "$JOB_ID"
curl -fsS -H "Authorization: Bearer $OMR_TOKEN" "$OMR_URL/v1/jobs/$JOB_ID" | jq .
```

Repeat the last command until `state` is `done`. If it reports a failure,
inspect `error` and `diagnostics` in the response and the service logs. Once
finished, download the result:

```sh
curl -fsS -H "Authorization: Bearer $OMR_TOKEN" \
  "$OMR_URL/v1/jobs/$JOB_ID/result" -o result.musicxml
unset OMR_TOKEN
```

## Data and licenses

The data directory contains the SQLite queue, uploads, temporary processing
files and results. Queued jobs survive restarts; jobs running during a restart
are marked interrupted. Terminal job data is pruned after seven days or when
retained terminal data exceeds 2 GiB. Protect and back up the directory as
appropriate for the scores it contains.

Ossia OMR is licensed under GPL-3.0-or-later. Audiveris is a separate dependency
licensed under [AGPL-3.0](https://github.com/Audiveris/audiveris/blob/master/LICENSE);
Tesseract language data and other dependencies have their own license terms.
Consult the upstream projects when redistributing them.
