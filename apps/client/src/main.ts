import { mount } from "svelte";
import App from "./App.svelte";
import { forwardErrorsToLog } from "./lib/log";
import "./app.css";

forwardErrorsToLog();

const target = document.getElementById("app");
if (!target) throw new Error("missing #app mount point");

export default mount(App, { target });
