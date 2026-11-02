# Forgejo primary, GitHub mirror

| Role | URL |
|------|-----|
| **Canonical repo** | http://192.168.50.18:3000/epsil0n/resume |
| **Issues / PRs** | http://192.168.50.18:3000/epsil0n/resume/issues |
| **GitHub mirror** (read-only visibility) | https://github.com/xxLYNX/resume |

## Clone

```bash
git clone http://192.168.50.18:3000/epsil0n/resume.git
cd resume
```

Use the [`fj`](https://codeberg.org/forgejo-contrib/forgejo-cli) CLI (`fj auth login --host http://192.168.50.18:3000`) or a personal access token for HTTPS pushes.

### Git credential helper (optional)

From the repo root, after `fj auth` has stored a token:

```bash
git config --global credential.http://192.168.50.18:3000.helper \
  "$(pwd)/scripts/forgejo-git-credential.sh"
```

Then `git push origin` works without embedding tokens in the remote URL.

## Remotes (this checkout)

| Remote | URL | Use |
|--------|-----|-----|
| `origin` | `http://192.168.50.18:3000/epsil0n/resume.git` | Push and fetch here |
| `github` | `git@github.com:xxLYNX/resume.git` | Manual mirror fallback |

```bash
git push origin master          # primary
./scripts/push-github-mirror.sh # if Forgejo push mirror is not configured yet
```

## Forgejo → GitHub push mirror

Configure on Forgejo (repo admin):

1. Open http://192.168.50.18:3000/epsil0n/resume/settings → **Mirror settings** → **Add push mirror**
2. Remote URL: `https://github.com/xxLYNX/resume.git`
3. Authentication: GitHub deploy key or fine-grained PAT with **Contents: write**
4. Enable sync on push

The Forgejo API may require admin privileges to create mirrors via token; use the web UI if API returns permission errors.

## Issues and PRs

Track work on **Forgejo only**. Do not open GitHub Issues for this project.

Create issues with `fj`:

```bash
fj --host http://192.168.50.18:3000 issue create "Title" --body-file body.md -r epsil0n/resume
```

## Cursor / gh

- Default push remote: `origin` (Forgejo).
- Use `gh` against GitHub only for mirror visibility if needed, not for issue workflow.
