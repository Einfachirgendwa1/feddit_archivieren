clean:
	rm -f core daemon.*

install:
	@cargo build
	@target/debug/client install
	@-cp feddit_archivieren.zsh /usr/share/zsh/site-functions/_feddit_archivieren.zsh

debug:
	@sudo feddit_archivieren maybe_kill
	@sudo feddit_archivieren update_local
	@sudo feddit_archivieren start
	@feddit_archivieren info
	@feddit_archivieren checkhealth
