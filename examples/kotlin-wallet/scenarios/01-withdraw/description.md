Add WalletService.withdraw for bank payouts

Payouts need to take money out of a wallet with the same checks as a transfer (owner and PIN).
The checks move into a private `authorize` helper used by both paths. Tests cover the happy path,
a wrong caller and an overdraw.
