hljs.registerLanguage("calmare", hljs => {
	const TERM = [];
	TERM.push({
		className: "number",
		begin: /0[xX][0-9A-Fa-f]+/,
	});
	TERM.push({
		className: "number",
		begin: /-?\d+(\.\d*)?((mm|ms|deg|mdeg)(\/s)?)?/,
	});
	TERM.push({
		className: "tag",
		begin: /\b(flag|system|var|global|name|bgm|magic|quest|shop|sound|town|battle|item|look_point|entrance|object|trigger|label|anim|chip|vis|fork|eff|eff_instance|menu|sepith|at_roll|placement|fn|emote|file|char_attr|char|field_party|party|custom)\[/,
		end: /]/,
		contains: TERM
	});
	TERM.push({
		className: "tag",
		begin: /\b(self|null|random)\b/,
	});
	TERM.push({
		className: "string",
		begin: /"/,
		end: /"/,
	});

	return {
		unicodeRegex: true,
		aliases: ["clm"],
		keywords: [],
		contains: [
			...TERM,
			{
				className: "function",
				begin: /\{/,
				end: /\}/,
				contains: [
					{
						className: "keyword",
						begin: /\{/,
						end: /\}/,
						contains: TERM,
					},
					{
						className: "keyword",
						begin: /#\d*[A-QS-Za-z]/,
					},
					{
						className: "keyword",
						begin: /#\d*R.*#/,
					}
				],
			},
			{
				className: "symbol",
				begin: /[-+*/%&^|<>!=~]+/,
			},
			hljs.COMMENT(/\/\//, /\n/)
		]
	}
});

for(let e of document.getElementsByClassName("language-clm"))
	hljs.highlightBlock(e);
