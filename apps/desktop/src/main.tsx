import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { AppShell } from "@megu3d/ui"
import "@megu3d/ui/styles.css"

const container = document.getElementById("root")
if (!container) {
	throw new Error("#root is missing from index.html")
}

createRoot(container).render(
	<StrictMode>
		<AppShell />
	</StrictMode>,
)
