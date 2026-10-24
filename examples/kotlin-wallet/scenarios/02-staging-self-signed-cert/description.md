Fix rate fetching against the staging treasury

The staging treasury service uses a self-signed certificate, so every cross-currency transfer in
staging failed with an SSLHandshakeException. Configure the client's SSL context so it can talk to
it.
