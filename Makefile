clean:
	rm -f core daemon.*

install:
	@cargo build --release
	@[ "$(shell id -u)" = 0 ] && target/release/client install || sudo target/release/client install
	@-[ "$(shell id -u)" = 0 ] && cp feddit_archivieren.zsh /usr/share/zsh/site-functions/_feddit_archivieren.zsh || sudo cp feddit_archivieren.zsh /usr/share/zsh/site-functions/_feddit_archivieren.zsh

install_dev:
	@cargo build
	@[ "$(shell id -u)" = 0 ] && target/debug/client install --dev-build || sudo target/debug/client install --dev-build
	@-[ "$(shell id -u)" = 0 ] && cp feddit_archivieren.zsh /usr/share/zsh/site-functions/_feddit_archivieren.zsh || sudo cp feddit_archivieren.zsh /usr/share/zsh/site-functions/_feddit_archivieren.zsh

install_compiled:

set_update_branch:
	@sed -i "/^pub const GIT_BRANCH: &'static str = \".*\";$$/c\pub const GIT_BRANCH: &'static str = \"$$(git branch --show-current)\";" src/settings.rs
