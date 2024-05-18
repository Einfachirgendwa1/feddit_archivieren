clean:
	rm -f core daemon.*

install:
	@cargo build
	@target/debug/client --subcommand install
	@-cp feddit_archivieren.zsh /usr/share/zsh/site-functions/_feddit_archivieren.zsh
