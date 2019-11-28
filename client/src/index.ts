import * as THREE from "three";
import { models } from "./loader";
import { scene } from "./scene";
import { createText } from "./text";

let aspect = window.innerHeight / window.innerWidth;

let view = 800;
let camera = new THREE.OrthographicCamera(-view, view, view * aspect, -view * aspect, 1, 1000);
camera.position.set(400, 400, 800);
camera.lookAt(400, 400, 0);

{
  const near = 806;
  const far = 890;
  const color = 0x0000f;
  scene.fog = new THREE.Fog(color, near, far);
  scene.background = new THREE.Color(color);
}

{
  let material = new THREE.LineBasicMaterial({ color: 0x0000ff });
  let geometry = new THREE.Geometry();
  geometry.vertices.push(new THREE.Vector3(0, 0, 0));
  geometry.vertices.push(new THREE.Vector3(0, 800, 0));
  geometry.vertices.push(new THREE.Vector3(800, 800, 0));
  geometry.vertices.push(new THREE.Vector3(800, 0, 0));
  geometry.vertices.push(new THREE.Vector3(0, 0, 0));
  let line = new THREE.Line(geometry, material);
  scene.add(line);
}

{
  let light = new THREE.AmbientLight(0x404040); // soft white light
  scene.add(light);
}
{
  const light = new THREE.DirectionalLight(0xffffff - 0x404040, 1);
  light.position.set(100, 100, 100);
  scene.add(light);
}

let bulletGeometry = new THREE.ConeGeometry(3, 100, 3);
let bulletMaterial = new THREE.MeshLambertMaterial({ color: 0x00aa00 });
let healthMaterial = new THREE.MeshLambertMaterial({ color: 0x88ff00 });
let manaMaterial = new THREE.MeshLambertMaterial({ color: 0x00bbff });
let cooldownMaterial = new THREE.MeshLambertMaterial({ color: 0xbc7a51 });

let renderer = new THREE.WebGLRenderer({ antialias: false, logarithmicDepthBuffer: true });
renderer.setSize(window.innerWidth, window.innerHeight);
document.body.appendChild(renderer.domElement);

const sprites: { [key: string]: THREE.Object3D } = {};
const bullets: { [key: string]: THREE.Mesh } = {};

let ws = new WebSocket("ws://" + window.location.host + "/ws/");
let opened = false;
let myid = 0;

document.getElementById("username").focus();
document.getElementById("username").addEventListener("keydown", e => {
  if (e.keyCode == 13 && opened) {
    send({ Spawn: [(document.getElementById("username") as HTMLInputElement).value, (document.getElementById("class") as HTMLSelectElement).value] });
    document.getElementById("login").style.display = "none";
  }
});
ws.addEventListener("open", () => {
  opened = true;
  document.getElementById("status").innerText = "Press enter to play";
});
ws.addEventListener("close", () => (opened = false));
ws.addEventListener("message", e => {
  const m = JSON.parse(e.data);
  if (m.you) myid = m.you;
  if (m.death) {
    let group = sprites[m.death];
    for (let i = group.children.length - 1; i >= 0; i--) {
      group.remove(group.children[i]);
    }
    scene.remove(group);
    delete sprites[m.death];

    if (m.death == myid) {
      document.getElementById("login").style.display = "block";
      document.getElementById("username").focus();
      camera.position.x = 400;
      camera.position.y = 400;
    }
  }
  if (m.players) {
    m.players.forEach((p: any) => {
      if (!sprites[p.id]) {
        let container = new THREE.Group();
        {
          let c = p.class == "Quickshot";
          let mesh = models[c ? "spaceCraft1" : "spaceCraft3"].clone();
          ((mesh.children[c ? 1 : 0] as THREE.Mesh).material as THREE.MeshStandardMaterial).color.setHex(0x00ff00);

          mesh.scale.x = 600;
          mesh.scale.y = 600;
          mesh.scale.z = 600;
          mesh.rotation.x = Math.PI / 2;
          mesh.position.set(0, 0, 0);
          container.add(mesh);
        }
        {
          let ring = new THREE.RingGeometry(68, 80, 10, 8, Math.PI, Math.PI / 2);
          let mesh = new THREE.Mesh(ring, healthMaterial);
          mesh.position.set(0, 0, 0);
          container.add(mesh);
        }
        {
          let ring = new THREE.RingGeometry(68, 80, 10, 8, 1.5 * Math.PI, Math.PI / 2);
          let mesh = new THREE.Mesh(ring, manaMaterial);
          mesh.position.set(0, 0, 0);
          container.add(mesh);
        }
        {
          let ring = new THREE.RingGeometry(80, 92, 10, 8, (4 / 3) * Math.PI, Math.PI / 3);
          let mesh = new THREE.Mesh(ring, cooldownMaterial);
          mesh.position.set(0, 0, 0);
          container.add(mesh);
        }
        if (p.name) {
          let text = createText(p.name, 30);
          text.position.y = 87;
          text.position.z = 1;
          container.add(text);
        }
        scene.add(container);
        sprites[p.id] = container;
      }
      if (p.id == myid) {
        camera.position.x = p.pos[0];
        camera.position.y = p.pos[1];
      }
      sprites[p.id].position.x = p.pos[0];
      sprites[p.id].position.y = p.pos[1];

      {
        let ring = new THREE.RingGeometry(68, 80, 10, 8, Math.PI * (1.5 - p.health / 510), (Math.PI * p.health) / 510);
        let geometry = (sprites[p.id].children[1] as THREE.Mesh).geometry as THREE.Geometry;
        geometry.vertices = ring.vertices;
        geometry.verticesNeedUpdate = true;
        geometry.elementsNeedUpdate = true;
      }
      {
        let ring = new THREE.RingGeometry(68, 80, 10, 8, 1.5 * Math.PI, (Math.PI * p.mana) / 510);
        let geometry = (sprites[p.id].children[2] as THREE.Mesh).geometry as THREE.Geometry;
        geometry.vertices = ring.vertices;
        geometry.verticesNeedUpdate = true;
        geometry.elementsNeedUpdate = true;
      }
      {
        let cooldown = p.class == "Quickshot" ? 750 : 1000;
        let timeDiff = Math.min(p.shot_time, cooldown);
        if (timeDiff == 0) {
          sprites[p.id].children[3].visible = false;
        } else {
          sprites[p.id].children[3].visible = true;
          let ring = new THREE.RingGeometry(80, 92, 10, 8, (4 / 3) * Math.PI, ((Math.PI / 3) * timeDiff) / cooldown);
          let geometry = (sprites[p.id].children[3] as THREE.Mesh).geometry as THREE.Geometry;
          geometry.vertices = ring.vertices;
          geometry.verticesNeedUpdate = true;
          geometry.elementsNeedUpdate = true;
        }
      }
      sprites[p.id].children[0].rotation.y = -p.angle;
    });
  }
  if (m.bullets) {
    let marked: { [key: string]: boolean } = {};
    for (let key in bullets) {
      marked[key] = false;
    }
    m.bullets.forEach((b: any) => {
      if (!bullets[b.id]) {
        let mesh = new THREE.Mesh(bulletGeometry, bulletMaterial);
        scene.add(mesh);

        mesh.rotation.z = Math.PI - Math.atan2(b.vel[0], b.vel[1]);
        bullets[b.id] = mesh;
      }
      bullets[b.id].position.x = b.pos[0];
      bullets[b.id].position.y = b.pos[1];
      marked[b.id] = true;
    });
    for (let key in bullets) {
      if (!marked[key]) {
        scene.remove(bullets[key]);
        delete bullets[key];
      }
    }
  }
  renderer.render(scene, camera);
});

function send(m: any) {
  if (opened) ws.send(JSON.stringify(m));
}
window.addEventListener("mousemove", e => {
  let a = Math.atan2(e.clientX - window.innerWidth / 2, window.innerHeight / 2 - e.clientY);
  if (a < 0) a += 2 * Math.PI;
  send({ Angle: a });
});
window.addEventListener("keydown", e => {
  if (e.keyCode == 83) send({ Split: true });
});
window.addEventListener("keyup", e => {
  if (e.keyCode == 83) send({ Split: false });
});
window.addEventListener("mousedown", () => {
  send({ Click: true });
});
window.addEventListener("mouseup", () => {
  send({ Click: false });
});
window.addEventListener("resize", () => {
  aspect = window.innerHeight / window.innerWidth;
  camera.top = view * aspect;
  camera.bottom = -view * aspect;
  camera.updateProjectionMatrix();
  renderer.setSize(window.innerWidth, window.innerHeight);
});
