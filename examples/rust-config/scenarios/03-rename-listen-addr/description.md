Rename listen_addr to bind and default to port 3000

Every other service in the fleet uses `bind` and port 3000, which keeps the ingress config
uniform. Updated the defaults, tests and README.
