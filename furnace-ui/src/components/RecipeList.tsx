import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Recipe = {
  name: string;
  site: string;
}

export default function RecipeList() {
  const [recipes, setRecipes] = useState<Recipe[]>([]);

  const fetchRecipes = async () => {
    try {
      const result = await invoke<Recipe[]>("recipe_list");
      setRecipes(result);
    } catch (e) {
      console.error("Error fetching recipes:", e);
    }
  };

  return (
    <div>
      <button onClick={fetchRecipes}>Fetch Recipes</button>
      <ul>
        {recipes.map((recipe, index) => (
          <li key={index}>{recipe.name}</li>
        ))}
      </ul>
    </div>
  );
}