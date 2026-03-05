#toolkit #security
# Replace unchecked arithmetic in wallet seed increment and offer delta calculation

Wallet seed increment now uses checked_add and returns an error on overflow instead of silently wrapping to zero. Offer delta calculation replaces unsafe `as i128` casts with `i128::try_from` and uses checked arithmetic to prevent silent truncation and overflow. Addresses audit finding Issue AL (Least Authority).

PR: https://github.com/midnightntwrk/midnight-node/pull/860
Ticket: https://shielded.atlassian.net/browse/PM-20017
Ticket: https://shielded.atlassian.net/browse/PM-20206
