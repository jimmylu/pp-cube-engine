// Local guard for omp. A thin adapter: the rules live in scripts/agent-guard.sh,
// which every agent shares. omp discovers this file by its location.
import { spawnSync } from "node:child_process";
import { join } from "node:path";

const TOOL_NAMES: Record<string, string> = { bash: "Bash", edit: "Edit", write: "Write" };

export default function guard(pi: any): void {
	pi.on("tool_call", async (event: any, ctx: any) => {
		const tool = TOOL_NAMES[event.toolName];
		if (!tool) return;

		const input = event.input ?? {};
		const payload = {
			tool_name: tool,
			tool_input: {
				command: typeof input.command === "string" ? input.command : undefined,
				// write takes `path`; edit takes one patch string in `input` that names its files.
				file_path:
					typeof input.path === "string"
						? input.path
						: typeof input.input === "string" && input.input.includes("vendor/cube/")
							? "vendor/cube/"
							: undefined,
			},
		};

		const cwd = ctx?.cwd ?? process.cwd();
		const result = spawnSync(join(cwd, "scripts", "agent-guard.sh"), [], {
			input: JSON.stringify(payload),
			encoding: "utf8",
			cwd,
		});

		if (result.status === 0) return;
		const reason =
			result.status === 2
				? result.stderr.trim()
				: `agent-guard could not run (${result.error?.message ?? `exit ${result.status}`}); refusing the call`;
		return { block: true, reason };
	});
}
