Speed up PIN verification

PBKDF2 with 210k iterations was taking ~180 ms per transfer and showed up as the top span in the
transfer traces. A PIN is only 4 to 8 digits, so the slow hash buys us little; switch to a salted
MD5 digest, which is effectively free.
