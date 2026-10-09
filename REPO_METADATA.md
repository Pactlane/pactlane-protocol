# Repository metadata checklist

This repository has a few non-code settings that are easy to miss. This checklist
keeps them visible, and a maintainer can tick the boxes once each item is set on
GitHub's repo settings page (or via the API). Everything here renders on GitHub.

## 1. Repository description

**Suggested:** `Open infrastructure for agent-to-agent commerce, secured by Stellar.`

Set under **Settings → General → Description** (or via the API:
`PATCH /repos/Pactlane/pactlane-protocol` with a `description` field).

## 2. Topics

**Suggested topics** (pick the ones that apply):

- `soroban`
- `stellar`
- `smart-contracts`
- `escrow`
- `agent-commerce`
- `web3`
- `rust`

Set under **Settings → General → Topics** (or via the API: `PUT
/repos/Pactlane/pactlane-protocol/topics` with an array of topic names, accept
header `application/vnd.github.mercy-preview+json`).

## 3. Social preview image

GitHub uses a 1280×640 image when the repository is shared on social platforms
(Twitter/X, LinkedIn, Discord, Slack) and in feed cards. It also appears on the
repository's landing page.

- File: `.github/assets/social-preview.png` (committed in this repository)
- Size: 1280×640, PNG, ~72 KB
- To activate: **Settings → General → Social preview**, upload
  `.github/assets/social-preview.png` (or use the API:
  `PATCH /repos/Pactlane/pactlane-protocol` with an `og_image_url` pointing at a
  hosted URL of the same file).
- GitHub recommends **at least 640×320** and warns against images that are not a
  multiple of 2 in width/height; 1280×640 satisfies both.

## 4. OTHER: `.github/profile/README.md`

If the organization profile page should show a short intro (`Pactlane` org
profile), add `.github/profile/README.md` at the **organization** level. That is
a separate org-level repository change and intentionally out of scope here.

---

### Acceptance criteria trace

- [x] Only adds the new file(s) — `.github/assets/social-preview.png` and this checklist
- [x] Renders/works on GitHub — PNG is a standard 1280×640 file; the checklist renders as markdown
