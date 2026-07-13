// Applied-filter chip derivation: a pure view over `filterText` (argument-driven,
// no DOM, no state — the shape of query.ts). The surface-aware parser is the only
// key-set authority and `tokenizeQuery` the only tokenization authority; this
// module never re-lists keys or reimplements quoting/negation splitting.

import { parseSearchQueryFor, type QuerySurface, tokenizeQuery } from "./query";

/** One removable chip: a parsed field clause plus the index of the raw token
 * that produced it in `tokenizeQuery(filterText)`. */
export interface FilterChip {
  tokenIndex: number;
  field: string;
  value: string;
  negate: boolean;
}

/**
 * The field-clause chips derived from `filterText` for `surface` — one per raw
 * token that parses as a supported field clause; a free-text token, or a clause
 * the surface parse drops with a diagnostic, produces no chip. Re-parsing each
 * token individually (rather than diffing the whole-string parse against its
 * clause list) keeps a repeated key's occurrences distinct: two `tag:a` tokens
 * are two chips, each carrying its own `tokenIndex`, so removing the second one
 * never touches the first.
 */
export function filterChipsFor(
  filterText: string,
  surface: QuerySurface,
): FilterChip[] {
  const chips: FilterChip[] = [];
  tokenizeQuery(filterText).forEach((raw, tokenIndex) => {
    const clause = parseSearchQueryFor(raw, surface).clauses[0];
    if (clause && clause.kind === "field") {
      chips.push({
        tokenIndex,
        field: clause.field,
        value: clause.value,
        negate: clause.negate,
      });
    }
  });
  return chips;
}

/** Remove the token at `tokenIndex` from `filterText`, preserving every other
 * token (free text and other qualifier clauses) in order. */
export function removeFilterChipToken(
  filterText: string,
  tokenIndex: number,
): string {
  const tokens = tokenizeQuery(filterText);
  tokens.splice(tokenIndex, 1);
  return tokens.join(" ");
}
