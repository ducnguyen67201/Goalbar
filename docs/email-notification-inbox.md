# Email notification inbox

Goalbar can organize conversation signals from X, Reddit, and LinkedIn without a platform developer application. The first connector reads the local Apple Mail inbox on macOS only when the user chooses **Check Apple Mail**.

## Setup

1. Add the relevant email account to Apple Mail and let Mail finish syncing.
2. On X, Reddit, and LinkedIn, enable email notifications for the conversation events you want to see: replies, comments, mentions, direct messages, or chat requests.
3. Keep those emails in the Apple Mail inbox until Goalbar has checked them.
4. In Goalbar, open **Inbox** and choose **Check Apple Mail**.
5. If macOS asks, allow Goalbar to control Mail. The permission can later be reviewed under **System Settings → Privacy & Security → Automation**.

## Local behavior

- Only sender domains belonging to X/Twitter, Reddit, or LinkedIn are accepted.
- A notification must also have a recognized conversation subject; newsletters and job or recommendation emails are ignored.
- Goalbar imports at most a bounded set of Apple Mail candidates per check and stores only a normalized excerpt plus notification metadata.
- The email message ID is used for idempotency, so checking again does not create duplicates.
- **New** is Goalbar's local unread state. Selecting a notification marks it read in Goalbar and does not alter the email or platform state.
- Links are accepted only for the matching platform, with tracking queries removed. Otherwise Goalbar opens that platform's safe notifications or messages page.

## Trust boundary

Email is a notification channel, not the complete source of truth. Excerpts may be incomplete, delayed, malformed, or attacker-controlled. Goalbar labels them accordingly, treats their text as untrusted when drafting, and asks the user to verify the complete thread on the platform.

For an email-derived conversation, Goalbar can draft and record approval for exact text, but it cannot send. The user copies the approved text and sends it on the platform website. No platform password, browser session, OAuth token, or complete raw email is passed to Codex or Claude.
