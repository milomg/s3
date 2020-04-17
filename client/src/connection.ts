let secure = (window.location.protocol.match(/s/g) || "").toString();
export let ws = new WebSocket(`ws${secure}://${window.location.host}/ws/`);
export let opened = false;

export function send(m: any) {
  if (opened) ws.send(JSON.stringify(m));
}

const connError = document.getElementById("error");
ws.onopen = () => {
  opened = true;
  document.getElementById("status").innerText = "Press enter to play";
  connError.style.visibility = "hidden";
};
ws.onclose = () => {
  opened = false;
  connError.style.visibility = "visible";
};
