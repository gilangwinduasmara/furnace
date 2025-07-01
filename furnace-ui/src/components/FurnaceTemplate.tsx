import { useState } from "react";

// Dummy data for recipes
const dummyRecipes = [
  { name: "Project Alpha", path: "/Users/you/alpha", php_version: "8.2", serve_with: "nginx", site: "alpha.test" },
  { name: "Project Beta", path: "/Users/you/beta", php_version: "8.1", serve_with: "apache", site: "beta.test" },
  { name: "Project Gamma", path: "/Users/you/gamma", php_version: "8.0", serve_with: "nginx", site: "gamma.test" },
];

export default function FurnaceTemplate() {
  const [selected, setSelected] = useState(0);

  return (
    <div style={{ display: "flex", height: "100vh", fontFamily: "sans-serif" }}>
      {/* Sidebar */}
      <aside style={{ width: 260, background: "#f8fafc", borderRight: "1px solid #e5e7eb", display: "flex", flexDirection: "column" }}>
        <div style={{ display: "flex", alignItems: "center", padding: 16, borderBottom: "1px solid #e5e7eb" }}>
          <span style={{ fontWeight: 600, fontSize: 18, flex: 1 }}>Recipes</span>
          <button style={{ fontSize: 22, fontWeight: 600, background: "#e0e7ef", border: "none", borderRadius: 6, width: 32, height: 32, cursor: "pointer" }}>+</button>
        </div>
        <ul style={{ listStyle: "none", margin: 0, padding: 0, flex: 1, overflowY: "auto" }}>
          {dummyRecipes.map((recipe, idx) => (
            <li key={recipe.name}>
              <button
                style={{
                  width: "100%",
                  textAlign: "left",
                  padding: "12px 20px",
                  background: idx === selected ? "#e0e7ef" : "transparent",
                  border: "none",
                  borderBottom: "1px solid #e5e7eb",
                  cursor: "pointer",
                  fontWeight: idx === selected ? 600 : 400,
                  color: idx === selected ? "#1e293b" : "#334155"
                }}
                onClick={() => setSelected(idx)}
              >
                {recipe.name}
              </button>
            </li>
          ))}
        </ul>
      </aside>
      {/* Content */}
      <main style={{ flex: 1, padding: 32 }}>
        <h2 style={{ fontSize: 28, fontWeight: 700, marginBottom: 8 }}>{dummyRecipes[selected].name}</h2>
        <div style={{ color: "#64748b", marginBottom: 16 }}>{dummyRecipes[selected].site}</div>
        <div style={{ background: "#f1f5f9", padding: 20, borderRadius: 8, maxWidth: 480 }}>
          <div><b>Path:</b> {dummyRecipes[selected].path}</div>
          <div><b>PHP Version:</b> {dummyRecipes[selected].php_version}</div>
          <div><b>Serve With:</b> {dummyRecipes[selected].serve_with}</div>
        </div>
      </main>
    </div>
  );
}
