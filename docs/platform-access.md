# Platform access ledger

This ledger is an implementation gate, not a hosted-infrastructure requirement. Users connect locally; the APIs still require a registered application and approved scopes.

The integrated browser does not need a Tagline developer application ID. It uses the user’s normal local website session when the site permits the embedded browser engine. This does not grant API access, remove platform restrictions, or authorize bulk automation. Tagline’s shipping website-collection policy is manual-only; use explicit capture or an official archive.

| Platform | App type                       | Required access                                                               | OAuth redirect                                 | Pricing/retention                          | Status           |
| -------- | ------------------------------ | ----------------------------------------------------------------------------- | ---------------------------------------------- | ------------------------------------------ | ---------------- |
| X        | Native/public OAuth 2.0 client | `tweet.read tweet.write users.read dm.read dm.write offline.access` as needed | registered loopback URI                        | pay-per-use; set a monthly budget          | approval_pending |
| Reddit   | Installed application          | `identity read history submit edit privatemessages` as needed                 | exact registered loopback URI                  | Data API approval and current terms review | approval_pending |
| LinkedIn | Native PKCE-enabled app        | Sign In, Share on LinkedIn, approved Community Management scopes              | random loopback port only when PKCE is enabled | restricted products/versioned APIs         | approval_pending |

## Required evidence before live acceptance

- Checked date and official source links.
- Application/client ID (non-secret).
- Approved products, scopes, tier, and test account.
- Exact redirect URI rules.
- X usage budget.
- Reddit data retention and AI-inference decision.
- LinkedIn native PKCE enablement and Community Management review state.

Never place a secret or token in this document.
