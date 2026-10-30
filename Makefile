# Local playground: disposable sample repositories under target/playground.
#   make playground         build the optimized binary and open a new playground
#   make playground-large   the same, plus 40 untracked files for Worktree scrolling
#   make playground-clean   delete every playground in target/playground
# Pass Gitview arguments with ARGS, for example: make playground ARGS="--run sh"

PLAYGROUND_PARENT := $(CURDIR)/target/playground
GITVIEW_BIN := $(CURDIR)/target/release/gitview
ARGS ?=

.PHONY: playground playground-large playground-clean

playground:
	cargo build --release --locked
	GITVIEW_PLAYGROUND_PARENT="$(PLAYGROUND_PARENT)" GITVIEW_BIN="$(GITVIEW_BIN)" \
		scripts/run-feature-playground.sh $(ARGS)

playground-large:
	cargo build --release --locked
	GITVIEW_PLAYGROUND_LARGE=1 GITVIEW_PLAYGROUND_PARENT="$(PLAYGROUND_PARENT)" \
		GITVIEW_BIN="$(GITVIEW_BIN)" scripts/run-feature-playground.sh $(ARGS)

playground-clean:
	rm -rf -- "$(PLAYGROUND_PARENT)"
	@echo "Removed $(PLAYGROUND_PARENT)"
