# Capability matrix

Runtime capability discovery is authoritative. This file records the expected public ceiling.

| Browser capability       | X                                               | Reddit      | LinkedIn    |
| ------------------------ | ----------------------------------------------- | ----------- | ----------- |
| Local navigation/sign-in | conditional on website embedded-engine behavior | conditional | conditional |
| Explicit preview/capture | supported                                       | supported   | supported   |
| Bounded auto-collection  | manual_only                                     | manual_only | manual_only |
| Fill draft               | manual_only                                     | manual_only | manual_only |
| Final Publish/Send click | unsupported                                     | unsupported | unsupported |
| Official archive import  | supported                                       | supported   | supported   |

| Browser inbox scan         | X           | Reddit      | LinkedIn    |
| -------------------------- | ----------- | ----------- | ----------- |
| Recent conversation rows   | conditional | conditional | conditional |
| Local unread/preview state | supported   | supported   | supported   |
| Complete historical inbox  | unsupported | unsupported | unsupported |
| Background scan            | unsupported | unsupported | unsupported |
| Send from Goalbar          | unsupported | unsupported | unsupported |

Browser inbox scans are conditional on the signed-in website exposing supported conversation rows to its embedded webview. They are explicit, bounded, and free of platform API fees; the platform website remains authoritative.

| Email-notification inbox | X           | Reddit      | LinkedIn    |
| ------------------------ | ----------- | ----------- | ----------- |
| Apple Mail import        | supported   | supported   | supported   |
| Reply/comment/mention    | conditional | conditional | conditional |
| Message signal           | conditional | conditional | conditional |
| Complete thread          | unsupported | unsupported | unsupported |
| Send from Goalbar        | unsupported | unsupported | unsupported |

Email notification support is conditional on the platform sending a recognizable notification to the local Apple Mail inbox. It is a user-triggered, free signal path rather than API parity; the platform website remains authoritative.

| Platform | Publish                         | Own content/metrics                | Replies/comments               | Direct messages                  |
| -------- | ------------------------------- | ---------------------------------- | ------------------------------ | -------------------------------- |
| X        | supported with granted scope    | supported according to access/tier | conditional on API eligibility | supported with granted DM scopes |
| Reddit   | approval_pending                | approval_pending                   | approval_pending               | approval_pending                 |
| LinkedIn | supported with approved product | restricted                         | restricted                     | unsupported; open in LinkedIn    |

States are `supported`, `unsupported`, `approval_pending`, or `unknown`. Unsupported actions must provide a reason and recovery action.

Browser policy states are `explicit_capture`, `bounded_collection`, `manual_only`, or `blocked`. They do not imply official API approval.
