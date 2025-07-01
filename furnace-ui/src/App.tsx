import { useState } from "react";
// import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import { Route, Routes } from "react-router-dom";
import "./App.css";
import FurnaceTemplate from "./components/FurnaceTemplate";
import Welcome from "./pages/Welcome";

function FurnaceStatus() {
  const [status, setStatus] = useState("");

  const checkStatus = async () => {
    try {
      const result = await invoke<string>("furnace_status");
      setStatus(result);
    } catch (e) {
      setStatus("Error: " + e);
    }
  };

  return (
    <div>
      <button onClick={checkStatus}>Check Furnace Status</button>
      <div>Status: {status}</div>
    </div>
  );
}


function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="h-screen w-screen bg-background">
      <Routes>
        <Route path="/" element={<Welcome />} />
        <Route path="/recipes" element={<FurnaceTemplate />} />
      </Routes>
    </main>
  );
}

export default App;
