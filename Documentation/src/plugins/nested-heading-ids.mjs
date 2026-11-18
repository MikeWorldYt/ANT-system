// Gives h3 ids their parent h2 slug (e.g. #keys-term) on reference pages whose subsections share short names.
const PAGES = ['/advanced/dictionaries.mdx'];

const slug = (text) =>
	text.trim().toLowerCase().replace(/[^\p{L}\p{N}\s-]/gu, '').replace(/\s+/g, '-');

const textOf = (node) =>
	node.type === 'text' ? node.value : (node.children ?? []).map(textOf).join('');

export default function nestedHeadingIds() {
	return (tree, file) => {
		const path = String(file.path ?? file.history?.[0] ?? '').replaceAll('\\', '/');
		if (!PAGES.some((p) => path.endsWith(p))) return;
		let parent = '';
		for (const node of tree.children) {
			if (node.type !== 'element') continue;
			if (node.tagName === 'h2') parent = slug(textOf(node));
			else if (node.tagName === 'h3' && parent) {
				node.properties = { ...node.properties, id: `${parent}-${slug(textOf(node))}` };
			}
		}
	};
}
