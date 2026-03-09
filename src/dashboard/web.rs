use std::path::Path;

use anyhow::{Context, Result};

use crate::graph::Graph;

/// Generate a self-contained HTML file with an interactive D3.js force-directed graph.
pub fn generate_web_dashboard(graph: &Graph, output_path: &Path) -> Result<()> {
    // Build nodes JSON
    let nodes_json: Vec<String> = graph
        .nodes
        .iter()
        .map(|n| {
            let provides = serde_json::to_string(&n.provides).unwrap_or_else(|_| "[]".into());
            let dependents = serde_json::to_string(&n.dependents).unwrap_or_else(|_| "[]".into());
            let depends_on = serde_json::to_string(&n.depends_on).unwrap_or_else(|_| "[]".into());
            format!(
                r#"{{"id":"{}","description":"{}","role":"{}","provides":{},"dependents":{},"depends_on":{}}}"#,
                escape_json(&n.name),
                escape_json(&n.description),
                n.role,
                provides,
                dependents,
                depends_on,
            )
        })
        .collect();

    // Build links JSON from depends_on edges
    let mut links_json: Vec<String> = Vec::new();
    for node in &graph.nodes {
        for dep in &node.depends_on {
            links_json.push(format!(
                r#"{{"source":"{}","target":"{}"}}"#,
                escape_json(&node.name),
                escape_json(dep),
            ));
        }
    }

    let nodes_str = format!("[{}]", nodes_json.join(","));
    let links_str = format!("[{}]", links_json.join(","));

    let html = generate_html(&nodes_str, &links_str, &graph.generated_at);

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    std::fs::write(output_path, html)
        .with_context(|| format!("writing {}", output_path.display()))?;

    Ok(())
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn generate_html(nodes_json: &str, links_json: &str, generated_at: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>archon dashboard</title>
<style>
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace; background: #0d1117; color: #c9d1d9; overflow: hidden; }}
#app {{ display: flex; height: 100vh; }}
#sidebar {{ width: 0; overflow: hidden; background: #161b22; border-right: 1px solid #30363d; transition: width 0.3s ease; flex-shrink: 0; }}
#sidebar.open {{ width: 320px; }}
#sidebar-content {{ width: 320px; padding: 20px; position: relative; }}
#sidebar h2 {{ color: #58a6ff; margin-bottom: 8px; font-size: 18px; }}
.role-badge {{ display: inline-block; padding: 2px 8px; border-radius: 3px; font-size: 12px; font-weight: 600; margin-bottom: 12px; }}
.desc {{ color: #8b949e; margin-bottom: 16px; line-height: 1.5; }}
.section {{ margin-bottom: 16px; }}
.section-title {{ color: #8b949e; font-size: 12px; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 6px; }}
.dep-item {{ padding: 4px 0; }}
.dep-item a {{ color: #58a6ff; text-decoration: none; cursor: pointer; }}
.dep-item a:hover {{ text-decoration: underline; }}
.close-btn {{ position: absolute; top: 10px; right: 10px; background: none; border: none; color: #8b949e; cursor: pointer; font-size: 18px; }}
#main {{ flex: 1; display: flex; flex-direction: column; }}
#toolbar {{ display: flex; align-items: center; gap: 8px; padding: 10px 16px; background: #161b22; border-bottom: 1px solid #30363d; }}
#toolbar .title {{ font-weight: 700; color: #58a6ff; margin-right: 16px; }}
.filter-btn {{ padding: 4px 10px; border-radius: 4px; border: 1px solid #30363d; background: transparent; color: #8b949e; cursor: pointer; font-size: 12px; transition: all 0.2s; }}
.filter-btn:hover {{ border-color: #58a6ff; color: #c9d1d9; }}
.filter-btn.active {{ background: #21262d; border-color: #58a6ff; color: #c9d1d9; }}
#search {{ margin-left: auto; padding: 4px 10px; border-radius: 4px; border: 1px solid #30363d; background: #0d1117; color: #c9d1d9; font-size: 13px; width: 200px; }}
#search:focus {{ outline: none; border-color: #58a6ff; }}
#graph {{ flex: 1; }}
svg {{ width: 100%; height: 100%; }}
.node circle {{ cursor: pointer; stroke-width: 2px; transition: r 0.2s; }}
.node circle:hover {{ filter: brightness(1.3); }}
.node text {{ fill: #c9d1d9; font-size: 11px; pointer-events: none; }}
.link {{ stroke: #30363d; stroke-opacity: 0.6; }}
.link-arrow {{ fill: #30363d; fill-opacity: 0.6; }}
.node.dimmed circle {{ opacity: 0.15; }}
.node.dimmed text {{ opacity: 0.15; }}
.link.dimmed {{ stroke-opacity: 0.05; }}
.link-arrow.dimmed {{ fill-opacity: 0.05; }}
.node.highlighted circle {{ stroke-width: 3px; filter: brightness(1.4); }}
.tooltip {{ position: absolute; background: #1c2128; border: 1px solid #30363d; border-radius: 6px; padding: 8px 12px; font-size: 12px; pointer-events: none; opacity: 0; transition: opacity 0.15s; z-index: 10; }}
#footer {{ padding: 6px 16px; background: #161b22; border-top: 1px solid #30363d; font-size: 11px; color: #484f58; text-align: right; }}
</style>
</head>
<body>
<div id="app">
  <div id="sidebar">
    <div id="sidebar-content">
      <button class="close-btn" onclick="closeSidebar()">&times;</button>
      <div id="sidebar-body"></div>
    </div>
  </div>
  <div id="main">
    <div id="toolbar">
      <span class="title">archon</span>
      <button class="filter-btn active" data-role="all" onclick="toggleFilter(this)">All</button>
      <button class="filter-btn" data-role="core" onclick="toggleFilter(this)">Core</button>
      <button class="filter-btn" data-role="extension" onclick="toggleFilter(this)">Extension</button>
      <button class="filter-btn" data-role="tool" onclick="toggleFilter(this)">Tool</button>
      <button class="filter-btn" data-role="service" onclick="toggleFilter(this)">Service</button>
      <button class="filter-btn" data-role="library" onclick="toggleFilter(this)">Library</button>
      <input id="search" type="text" placeholder="Search nodes..." oninput="onSearch(this.value)">
    </div>
    <div id="graph"></div>
    <div id="footer">Generated {generated_at} &middot; archon dashboard</div>
  </div>
</div>
<div class="tooltip" id="tooltip"></div>
<script src="https://d3js.org/d3.v7.min.js"></script>
<script>
const NODES = {nodes_json};
const LINKS = {links_json};

const ROLE_COLORS = {{
  core: '#22d3ee',
  extension: '#4ade80',
  tool: '#facc15',
  service: '#c084fc',
  library: '#60a5fa'
}};

let activeFilter = 'all';
let searchQuery = '';
let selectedNode = null;

const graphEl = document.getElementById('graph');
const width = graphEl.clientWidth;
const height = graphEl.clientHeight;

const svg = d3.select('#graph').append('svg');
const g = svg.append('g');

// Zoom
const zoom = d3.zoom().scaleExtent([0.2, 5]).on('zoom', (e) => g.attr('transform', e.transform));
svg.call(zoom);

// Arrow markers
svg.append('defs').selectAll('marker')
  .data(['arrow'])
  .join('marker')
  .attr('id', d => d)
  .attr('viewBox', '0 -5 10 10')
  .attr('refX', 20)
  .attr('refY', 0)
  .attr('markerWidth', 6)
  .attr('markerHeight', 6)
  .attr('orient', 'auto')
  .append('path')
  .attr('d', 'M0,-5L10,0L0,5')
  .attr('class', 'link-arrow');

const simulation = d3.forceSimulation(NODES)
  .force('link', d3.forceLink(LINKS).id(d => d.id).distance(120))
  .force('charge', d3.forceManyBody().strength(-400))
  .force('center', d3.forceCenter(width / 2, height / 2))
  .force('collision', d3.forceCollide().radius(40));

const link = g.append('g')
  .selectAll('line')
  .data(LINKS)
  .join('line')
  .attr('class', 'link')
  .attr('marker-end', 'url(#arrow)');

const node = g.append('g')
  .selectAll('g')
  .data(NODES)
  .join('g')
  .attr('class', 'node')
  .call(d3.drag()
    .on('start', dragStarted)
    .on('drag', dragged)
    .on('end', dragEnded));

node.append('circle')
  .attr('r', d => 8 + (d.dependents?.length || 0) * 2)
  .attr('fill', d => ROLE_COLORS[d.role] || '#8b949e')
  .attr('stroke', d => d3.color(ROLE_COLORS[d.role] || '#8b949e').darker(0.5));

node.append('text')
  .attr('dx', 14)
  .attr('dy', 4)
  .text(d => d.id);

// Tooltip
const tooltip = d3.select('#tooltip');
node.on('mouseover', (e, d) => {{
  const ttEl = document.getElementById('tooltip');
  // Use textContent for safe rendering
  ttEl.textContent = '';
  const nameDiv = document.createElement('div');
  nameDiv.className = 'tt-name';
  nameDiv.textContent = d.id;
  const roleDiv = document.createElement('div');
  roleDiv.className = 'tt-role';
  roleDiv.textContent = d.role;
  ttEl.appendChild(nameDiv);
  ttEl.appendChild(roleDiv);
  tooltip.style('opacity', 1)
    .style('left', (e.pageX + 12) + 'px')
    .style('top', (e.pageY - 10) + 'px');
}})
.on('mousemove', (e) => {{
  tooltip.style('left', (e.pageX + 12) + 'px').style('top', (e.pageY - 10) + 'px');
}})
.on('mouseout', () => tooltip.style('opacity', 0))
.on('click', (e, d) => openSidebar(d));

simulation.on('tick', () => {{
  link.attr('x1', d => d.source.x).attr('y1', d => d.source.y)
      .attr('x2', d => d.target.x).attr('y2', d => d.target.y);
  node.attr('transform', d => `translate(${{d.x}},${{d.y}})`);
}});

function dragStarted(e, d) {{
  if (!e.active) simulation.alphaTarget(0.3).restart();
  d.fx = d.x; d.fy = d.y;
}}
function dragged(e, d) {{ d.fx = e.x; d.fy = e.y; }}
function dragEnded(e, d) {{
  if (!e.active) simulation.alphaTarget(0);
  d.fx = null; d.fy = null;
}}

// Safely build sidebar DOM using textContent (no innerHTML with user data)
function openSidebar(d) {{
  selectedNode = d;
  const sb = document.getElementById('sidebar');
  const body = document.getElementById('sidebar-body');
  body.textContent = ''; // Clear safely

  const h2 = document.createElement('h2');
  h2.textContent = d.id;
  body.appendChild(h2);

  const badge = document.createElement('span');
  badge.className = 'role-badge';
  badge.style.background = (ROLE_COLORS[d.role] || '#8b949e') + '33';
  badge.style.color = ROLE_COLORS[d.role] || '#8b949e';
  badge.textContent = d.role;
  body.appendChild(badge);

  const desc = document.createElement('div');
  desc.className = 'desc';
  desc.textContent = d.description;
  body.appendChild(desc);

  // Provides
  if (d.provides && d.provides.length) {{
    const sec = makeSection('Provides');
    d.provides.forEach(p => {{
      const item = document.createElement('div');
      item.className = 'dep-item';
      item.textContent = '\u2022 ' + p;
      sec.appendChild(item);
    }});
    body.appendChild(sec);
  }}

  // Depends on
  const depsSec = makeSection('Depends on (' + (d.depends_on?.length || 0) + ')');
  if (d.depends_on && d.depends_on.length) {{
    d.depends_on.forEach(dep => {{
      const item = document.createElement('div');
      item.className = 'dep-item';
      const a = document.createElement('a');
      a.textContent = '\u2192 ' + dep;
      a.onclick = () => focusNode(dep);
      item.appendChild(a);
      depsSec.appendChild(item);
    }});
  }} else {{
    const item = document.createElement('div');
    item.className = 'dep-item';
    item.style.color = '#484f58';
    item.textContent = '(none)';
    depsSec.appendChild(item);
  }}
  body.appendChild(depsSec);

  // Depended by
  const rdepsSec = makeSection('Depended by (' + (d.dependents?.length || 0) + ')');
  if (d.dependents && d.dependents.length) {{
    d.dependents.forEach(dep => {{
      const item = document.createElement('div');
      item.className = 'dep-item';
      const a = document.createElement('a');
      a.textContent = '\u2190 ' + dep;
      a.onclick = () => focusNode(dep);
      item.appendChild(a);
      rdepsSec.appendChild(item);
    }});
  }} else {{
    const item = document.createElement('div');
    item.className = 'dep-item';
    item.style.color = '#484f58';
    item.textContent = '(none)';
    rdepsSec.appendChild(item);
  }}
  body.appendChild(rdepsSec);

  sb.classList.add('open');
  highlightConnections(d);
}}

function makeSection(title) {{
  const sec = document.createElement('div');
  sec.className = 'section';
  const t = document.createElement('div');
  t.className = 'section-title';
  t.textContent = title;
  sec.appendChild(t);
  return sec;
}}

function closeSidebar() {{
  document.getElementById('sidebar').classList.remove('open');
  selectedNode = null;
  node.classed('dimmed', false).classed('highlighted', false);
  link.classed('dimmed', false);
}}

function focusNode(id) {{
  const n = NODES.find(n => n.id === id);
  if (n) openSidebar(n);
}}

function highlightConnections(d) {{
  const connected = new Set([d.id]);
  (d.depends_on || []).forEach(dep => connected.add(dep));
  (d.dependents || []).forEach(dep => connected.add(dep));

  node.classed('dimmed', n => !connected.has(n.id));
  node.classed('highlighted', n => n.id === d.id);
  link.classed('dimmed', l => !connected.has(l.source.id) || !connected.has(l.target.id));
}}

function toggleFilter(btn) {{
  document.querySelectorAll('.filter-btn').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
  activeFilter = btn.dataset.role;
  applyFilters();
}}

function onSearch(q) {{
  searchQuery = q.toLowerCase();
  applyFilters();
}}

function applyFilters() {{
  node.each(function(d) {{
    const roleMatch = activeFilter === 'all' || d.role === activeFilter;
    const searchMatch = !searchQuery || d.id.toLowerCase().includes(searchQuery);
    const visible = roleMatch && searchMatch;
    d3.select(this).style('opacity', visible ? 1 : 0.1);
  }});
  link.style('opacity', l => {{
    const sVisible = (activeFilter === 'all' || l.source.role === activeFilter) && (!searchQuery || l.source.id.toLowerCase().includes(searchQuery));
    const tVisible = (activeFilter === 'all' || l.target.role === activeFilter) && (!searchQuery || l.target.id.toLowerCase().includes(searchQuery));
    return sVisible && tVisible ? 0.6 : 0.05;
  }});
}}

// Handle resize
window.addEventListener('resize', () => {{
  simulation.force('center', d3.forceCenter(
    document.getElementById('graph').clientWidth / 2,
    document.getElementById('graph').clientHeight / 2
  ));
  simulation.alpha(0.3).restart();
}});
</script>
</body>
</html>"##,
        nodes_json = nodes_json,
        links_json = links_json,
        generated_at = escape_html(generated_at),
    )
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
