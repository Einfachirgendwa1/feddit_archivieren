PROGRAM_NAME = feddit_tmp 

clean:
	@rm savefile.txt log.txt

build:
	@cargo build --release

setup:
	@mkdir -p ../feddit_archivieren_logs
	@cp Makefile ../feddit_archivieren_logs

copy:
	@cp target/release/$(PROGRAM_NAME) ../feddit_archivieren_logs

run:
	@cd ../feddit_archivieren_logs && ./$(PROGRAM_NAME)

install_current:
	@make setup
	@make build
	@make copy

update:
	@git pull
	@make install_current

install:
	@make update

uninstall:
	@rm -rf ../feddit_archivieren_logs

test:
	@make uninstall
	@make install_current
	@make run
