import { describe, expect, it } from "vitest"

import { fuzzyMatch, highlightRuns, rankItems } from "./fuzzy"

describe("fuzzy", () => {
	it("matches subsequences and reports hit positions", () => {
		const match = fuzzyMatch("tgl", "Toggle panel")
		expect(match?.indices).toEqual([0, 2, 4])
		expect(fuzzyMatch("zzz", "Toggle panel")).toBeNull()
	})

	it("treats an empty query as a match for everything", () => {
		expect(fuzzyMatch("  ", "anything")).toEqual({ score: 0, indices: [] })
	})

	it("prefers prefixes and consecutive runs", () => {
		const prefix = fuzzyMatch("undo", "Undo")
		const scattered = fuzzyMatch("undo", "Unsaved document order")
		expect(prefix).not.toBeNull()
		expect(scattered).not.toBeNull()
		if (prefix && scattered) expect(prefix.score).toBeGreaterThan(scattered.score)
	})

	it("ranks stably and drops non matches", () => {
		const items = ["Reset workspace layout", "Rendering", "Rigging"]
		const ranked = rankItems("rig", items, (item) => item)
		expect(ranked.map((entry) => entry.item)).toEqual(["Rigging"])
		expect(rankItems("", items, (item) => item)).toHaveLength(3)
	})

	it("builds highlight runs", () => {
		expect(highlightRuns("Undo", [0, 1])).toEqual([
			{ text: "Un", matched: true },
			{ text: "do", matched: false },
		])
		expect(highlightRuns("Undo", [])).toEqual([{ text: "Undo", matched: false }])
	})
})
