# The Bitbucket Pipe: the pr-guard image (binary and weights) plus the script that maps the pipe's variables to flags.
#   docker build --build-arg VERSION=0.1.0 -t ghcr.io/typestrict-lab/pr-guard-bitbucket-pipe:0.1.0 integrations/bitbucket
ARG VERSION=0.1.0
FROM ghcr.io/typestrict-lab/pr-guard:${VERSION}
COPY pipe.sh /pipe.sh
ENTRYPOINT ["/pipe.sh"]
CMD []
