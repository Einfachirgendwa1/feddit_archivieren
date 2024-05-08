local dap = require("dap")

dap.adapters.lldb = {
	type = "executable",
	command = "/usr/bin/lldb-vscode",
	name = "lldb",
}

dap.configurations.rust = {
	{
		name = "feddit_archivieren",
		type = "lldb",
		request = "launch",
		program = function()
			return vim.fn.getcwd() .. "/target/debug/feddit_archivieren"
		end,
		cwd = "${workspaceFolder}",
		stopOnEntry = false,
	},
}
