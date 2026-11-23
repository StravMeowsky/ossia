# Website deployment

The static files in `website/` are published to GitHub Pages by
`.github/workflows/pages.yml` when they change on `main`. The workflow can also
be run manually. Only `website/` is published; everything in that directory is
public. Replace the initial page with the official website when it is ready.

Configure the repository's Pages source as GitHub Actions and set the custom
domain. The `github-pages` environment should permit
deployments only from `main`.

In GitHub account settings, verify the domain under Pages using the TXT record
provided by GitHub, and keep that record in DNS. In the DNS provider, create a
DNS-only CNAME from the chosen subdomain to `micooz.github.io`. Configure the custom domain
in GitHub before adding the CNAME. Enable Enforce HTTPS in Pages when the
certificate is ready. GitHub Actions deployments do not require a CNAME file.

Check the deployment in Actions and Pages settings, then verify DNS and HTTPS,
replacing `example.com` with the configured domain:

```sh
dig +short CNAME example.com
curl -I https://example.com/
curl -I http://example.com/
```

The CNAME should resolve to `micooz.github.io.` and HTTP should redirect to
HTTPS once Enforce HTTPS is active.
