export type FuzzyMatch = { score: number; indices: number[] }

const BOUNDARIES = new Set([" ", ".", ":", "-", "/", "_", "("])

/**
 * Subsequence matcher tuned for command titles: word starts and consecutive
 * runs score higher, long gaps are penalized. Returns `null` when the query
 * does not match at all.
 */
export function fuzzyMatch(query: string, text: string): FuzzyMatch | null {
	const needle = query.trim().toLowerCase()
	if (needle.length === 0) return { score: 0, indices: [] }

	const haystack = text.toLowerCase()
	const indices: number[] = []
	let score = 0
	let cursor = 0
	let streak = 0

	for (const char of needle) {
		if (char === " ") continue
		const found = haystack.indexOf(char, cursor)
		if (found === -1) return null
		const previous = found > 0 ? haystack[found - 1] : undefined
		const atBoundary = found === 0 || (previous !== undefined && BOUNDARIES.has(previous))
		streak = indices[indices.length - 1] === found - 1 ? streak + 1 : 0
		score += 12 + streak * 6 + (atBoundary ? 8 : 0) - Math.min(found - cursor, 6)
		indices.push(found)
		cursor = found + 1
	}

	return { score, indices }
}

/** Stable ranking: score first, original order as the tiebreaker. */
export function rankItems<T>(
	query: string,
	items: readonly T[],
	toText: (item: T) => string,
): Array<{ item: T; match: FuzzyMatch }> {
	const ranked: Array<{ item: T; match: FuzzyMatch; order: number }> = []
	items.forEach((item, order) => {
		const match = fuzzyMatch(query, toText(item))
		if (match) ranked.push({ item, match, order })
	})
	ranked.sort((left, right) => right.match.score - left.match.score || left.order - right.order)
	return ranked.map(({ item, match }) => ({ item, match }))
}

/** Splits text into matched/unmatched runs so the palette can highlight hits. */
export function highlightRuns(
	text: string,
	indices: number[],
): Array<{ text: string; matched: boolean }> {
	if (indices.length === 0) return [{ text, matched: false }]
	const marked = new Set(indices)
	const runs: Array<{ text: string; matched: boolean }> = []
	for (let index = 0; index < text.length; index += 1) {
		const matched = marked.has(index)
		const character = text.charAt(index)
		const last = runs[runs.length - 1]
		if (last && last.matched === matched) last.text += character
		else runs.push({ text: character, matched })
	}
	return runs
}
