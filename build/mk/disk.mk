# Transactional disk-image generation for the Redox installer.
# Build into a temporary file and publish only after the installer succeeds.

$(BUILD)/harddrive.img: $(FSTOOLS) $(REPO_TAG)
ifeq ($(FSTOOLS_IN_PODMAN),1)
	$(PODMAN_RUN) make $@
else
	@set -eu; \
	mkdir -p "$(BUILD)"; \
	partial="$@.partial"; \
	cleanup() { rm -f "$$partial"; }; \
	trap cleanup EXIT; \
	trap 'exit 1' HUP INT TERM; \
	filesystem_size="$(FILESYSTEM_SIZE)"; \
	if [ -z "$$filesystem_size" ]; then filesystem_size="$$("$(INSTALLER)" --filesystem-size -c "$(FILESYSTEM_CONFIG)")"; fi; \
	case "$$filesystem_size" in ''|*[!0-9]*) echo "Invalid filesystem size in MB: $$filesystem_size" >&2; exit 1;; esac; \
	rm -f "$$partial"; \
	truncate -s "$$filesystem_size"m "$$partial"; \
	umask 002; \
	"$(INSTALLER)" $(INSTALLER_OPTS) -c "$(FILESYSTEM_CONFIG)" "$$partial"; \
	test -s "$$partial"; \
	mv -f "$$partial" "$@"; \
	trap - EXIT HUP INT TERM; \
	echo "Created $@ ($$(du -h "$@" | cut -f1))"
endif

$(BUILD)/redox-live.iso: $(FSTOOLS) $(REPO_TAG) redox.ipxe
ifeq ($(FSTOOLS_IN_PODMAN),1)
	$(PODMAN_RUN) make $@
else
	@set -eu; \
	mkdir -p "$(BUILD)"; \
	partial="$@.partial"; \
	cleanup() { rm -f "$$partial"; }; \
	trap cleanup EXIT HUP INT TERM; \
	filesystem_size="$(FILESYSTEM_SIZE)"; \
	if [ -z "$$filesystem_size" ]; then filesystem_size="$$("$(INSTALLER)" --filesystem-size -c "$(FILESYSTEM_CONFIG)")"; fi; \
	case "$$filesystem_size" in ''|*[!0-9]*) echo "Invalid filesystem size in MB: $$filesystem_size" >&2; exit 1;; esac; \
	rm -f "$$partial"; \
	truncate -s "$$filesystem_size"m "$$partial"; \
	umask 002; \
	"$(INSTALLER)" $(INSTALLER_OPTS) -c "$(FILESYSTEM_CONFIG)" --write-bootloader="$(BUILD)/bootloader-live.efi" --live "$$partial"; \
	test -s "$$partial"; \
	mv -f "$$partial" "$@"; \
	cp redox.ipxe "$(BUILD)/redox.ipxe"; \
	trap - EXIT HUP INT TERM; \
	echo "Created $@ ($$(du -h "$@" | cut -f1))"
endif

$(BUILD)/filesystem.img: $(FSTOOLS) $(REPO_TAG)
ifeq ($(FSTOOLS_IN_PODMAN),1)
	$(PODMAN_RUN) make $@
else
	mkdir -p $(BUILD)
	-$(FUMOUNT) $(MOUNT_DIR) || true
	rm -rf $@.partial $(MOUNT_DIR)
	-$(FUMOUNT) /tmp/redox_installer || true
	FILESYSTEM_SIZE=$(FILESYSTEM_SIZE) && \
	if [ -z "$$FILESYSTEM_SIZE" ] ; then \
	FILESYSTEM_SIZE=$(shell $(INSTALLER) --filesystem-size -c $(FILESYSTEM_CONFIG)); \
	fi && \
	truncate -s "$$FILESYSTEM_SIZE"m $@.partial
	$(REDOXFS_MKFS) $(REDOXFS_MKFS_FLAGS) $@.partial
	mkdir -p $(MOUNT_DIR)
	$(REDOXFS) $@.partial $(MOUNT_DIR)
	sleep 1
	pgrep redoxfs
	umask 002 && $(INSTALLER) $(INSTALLER_OPTS) -c $(FILESYSTEM_CONFIG) $(MOUNT_DIR)
	sync
	-$(FUMOUNT) $(MOUNT_DIR) || true
	rm -rf $(MOUNT_DIR)
	test -s $@.partial
	mv -f $@.partial $@
endif

mount: $(FSTOOLS) FORCE
ifeq ($(FSTOOLS_IN_PODMAN),1)
	$(PODMAN_RUN) make $@
else
	@mkdir -p $(MOUNT_DIR)
	$(REDOXFS) $(BUILD)/harddrive.img $(MOUNT_DIR)
	@sleep 2
	@echo "\033[1;36;49mharddrive.img mounted ($$(pgrep redoxfs))\033[0m"
endif

mount_extra: $(FSTOOLS) FORCE
ifeq ($(FSTOOLS_IN_PODMAN),1)
	$(PODMAN_RUN) make $@
else
	@mkdir -p $(MOUNT_DIR)
	$(REDOXFS) $(BUILD)/extra.img $(MOUNT_DIR)
	@sleep 2
	@echo "\033[1;36;49mextra.img mounted ($$(pgrep redoxfs))\033[0m"
endif

mount_live: $(FSTOOLS) FORCE
ifeq ($(FSTOOLS_IN_PODMAN),1)
	$(PODMAN_RUN) make $@
else
	@mkdir -p $(MOUNT_DIR)
	$(REDOXFS) $(BUILD)/redox-live.iso $(MOUNT_DIR)
	@sleep 2
	@echo "\033[1;36;49mredox-live.iso mounted ($$(pgrep redoxfs))\033[0m"
endif

unmount: FORCE
ifeq ($(FSTOOLS_IN_PODMAN),1)
	$(PODMAN_RUN) make $@
else
	@sync
ifneq ($(wildcard $(MOUNT_DIR)),)
	$(FUMOUNT) $(MOUNT_DIR)
	@rm -rf $(MOUNT_DIR)
	@echo "\033[1;36;49mFilesystem unmounted\033[0m"
else
	@echo "\033[1;36;49mDisk is not mounted\033[0m"
endif
	@-$(FUMOUNT) /tmp/redox_installer 2>/dev/null || true
endif
