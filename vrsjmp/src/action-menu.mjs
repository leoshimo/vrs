export function menuEntries(item, query = "") {
    const words = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
    const entries = item.actions.map((command, index) => ({ command, index }));
    const matches = entries.filter(({ command }) => words.every(word => {
        let position = 0;
        const letters = Array.from(word);
        for (const letter of command.title.toLocaleLowerCase()) {
            if (letter === letters[position]) position++;
        }
        return position === letters.length;
    }));
    // Keep dispatch indices intact when grouping the default action above the others.
    return words.length ? matches : matches.sort((a, b) => Number(Boolean(b.command.primary)) - Number(Boolean(a.command.primary)));
}

export function defaultMenuSelection(entries, query = "") {
    if (query.trim()) return 0;
    const secondary = entries.findIndex(({ command }) => !command.primary);
    return secondary < 0 ? 0 : secondary;
}
