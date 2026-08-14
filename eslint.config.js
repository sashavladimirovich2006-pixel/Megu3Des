import js from "@eslint/js"
import tseslint from "typescript-eslint"

export default tseslint.config(
	{ ignores: ["**/dist/**", "**/target/**", "**/src/generated/**", "**/node_modules/**"] },
	js.configs.recommended,
	...tseslint.configs.recommended,
	{
		rules: {
			"@typescript-eslint/consistent-type-imports": "error",
			"no-console": ["warn", { allow: ["warn", "error"] }],
		},
	},
)
