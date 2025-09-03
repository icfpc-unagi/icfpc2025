%: %.Dockerfile
	$(MAKE) $*@build

server: server.Dockerfile
	cd .. && make secrets
	$(MAKE) server@build

%@build: FORCE
	cd .. && docker build --build-arg="UNAGI_PASSWORD=${UNAGI_PASSWORD}" --platform=linux/amd64 \
		--add-host "archive.ubuntu.com:$$(dig +short A asia-northeast1.gce.archive.ubuntu.com | head -n1)" \
		-f docker/$*.Dockerfile -t icfpc-unagi/$* .

.PHONY: FORCE
FORCE:
