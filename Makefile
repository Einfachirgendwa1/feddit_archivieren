clean:
	rm -f core daemon.*

install:
	@cargo build
	@[ "$(shell id -u)" = 0 ] && target/debug/client install || sudo target/debug/client install
	@-[ "$(shell id -u)" = 0 ] && cp feddit_archivieren.zsh /usr/share/zsh/site-functions/_feddit_archivieren.zsh || sudo cp feddit_archivieren.zsh /usr/share/zsh/site-functions/_feddit_archivieren.zsh
