import * as THREE from "three";
import { EffectComposer } from "three/examples/jsm/postprocessing/EffectComposer";
import { UnrealBloomPass } from "three/examples/jsm/postprocessing/UnrealBloomPass";
import { RenderPass } from "three/examples/jsm/postprocessing/RenderPass";
import { ShaderPass } from "three/examples/jsm/postprocessing/ShaderPass";
import { models } from "./loader";
import { scene } from "./scene";
import { createText } from "./text";
import { ws } from "./connection";
import "./controls";

let aspect = window.innerHeight / window.innerWidth;

let view = 800;
let camera = new THREE.OrthographicCamera(-view, view, view * aspect, -view * aspect, 1, 1000);
camera.position.set(400, 400, 800);
camera.lookAt(400, 400, 0);

let bulletGeometry = new THREE.ConeBufferGeometry(1.5, 50, 4);
let bulletMaterial = new THREE.MeshLambertMaterial({ color: 0x81e1fc });
let healthMaterial = new THREE.MeshLambertMaterial({ color: 0x3fab00 });
let manaMaterial = new THREE.MeshLambertMaterial({ color: 0x00bbff });
let cooldownMaterial = new THREE.MeshLambertMaterial({ color: 0x694129 });

let wormholegeometry = new THREE.IcosahedronBufferGeometry(50, 3);
let wormholematerials = [0x00ff00, 0xffff00, 0xff0000].map((c) => new THREE.MeshLambertMaterial({ color: c }));

let bossmaterial = new THREE.MeshLambertMaterial({ color: 0xffff00 });
let bosshealthgeometry = new THREE.PlaneBufferGeometry(100, 20);
let bosshealthmaterial = new THREE.MeshLambertMaterial({ color: 0xff0000 });

let renderer = new THREE.WebGLRenderer({ logarithmicDepthBuffer: true });
renderer.setPixelRatio(window.devicePixelRatio || 1);
renderer.toneMapping = THREE.NoToneMapping;
renderer.setSize(window.innerWidth, window.innerHeight);
document.body.appendChild(renderer.domElement);

let bloomPass = new UnrealBloomPass(new THREE.Vector2(window.innerWidth, window.innerHeight), 1.5, 0.5, 0);
let renderPass = new RenderPass(scene, camera);

let bloomComposer = new EffectComposer(renderer);
bloomComposer.renderToScreen = false;
bloomComposer.addPass(renderPass);
bloomComposer.addPass(bloomPass);

const sprites: { [key: string]: THREE.Object3D } = {};
const bullets: { [key: string]: THREE.Mesh } = {};
let wormholes: THREE.Mesh[] = [];
let boss: THREE.Object3D | undefined = undefined;

let uiElements: THREE.Mesh[] = [];
let uiMaterials: (THREE.Material | THREE.Material[])[] = [];
let darkMaterial = new THREE.MeshBasicMaterial({ color: 0x000000 });

function darkenUI() {
  for (let el in uiElements) {
    uiMaterials[el] = uiElements[el].material;
    uiElements[el].material = darkMaterial;
  }
}

function restoreUI() {
  for (let el in uiElements) {
    uiElements[el].material = uiMaterials[el];
    uiMaterials[el] = undefined;
  }
}

var finalPass = new ShaderPass(
  new THREE.ShaderMaterial({
    uniforms: {
      baseTexture: { value: null },
      bloomTexture: { value: bloomComposer.renderTarget2.texture },
    },
    vertexShader: `varying vec2 vUv;

    void main() {

      vUv = uv;

      gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );

    }`,
    fragmentShader: `uniform sampler2D baseTexture;
    uniform sampler2D bloomTexture;

    varying vec2 vUv;

    vec4 getTexture( sampler2D texelToLinearTexture ) {

      return mapTexelToLinear( texture2D( texelToLinearTexture , vUv ) );

    }

    void main() {

      gl_FragColor = ( getTexture( baseTexture ) + vec4( 1.0 ) * getTexture( bloomTexture ) );

    }`,
    defines: {},
  }),
  "baseTexture"
);
finalPass.needsSwap = true;

var finalComposer = new EffectComposer(renderer);
finalComposer.addPass(renderPass);
finalComposer.addPass(finalPass);
let myid = 0;

ws.onmessage = (e) => {
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
  if (m.clear) {
    for (let w of wormholes) {
      scene.remove(w);
    }
    wormholes = [];
    for (let p in sprites) {
      let group = sprites[p];
      for (let i = group.children.length - 1; i >= 0; i--) {
        group.remove(group.children[i]);
      }
      scene.remove(sprites[p]);
      sprites[p].visible = false;
      delete sprites[p];
    }
    for (let b in bullets) {
      scene.remove(bullets[b]);
      delete sprites[b];
    }
  }
  if (m.wormhole) {
    let sphere = new THREE.Mesh(wormholegeometry, wormholematerials[m.wormhole.color]);
    sphere.position.x = m.wormhole.pos[0];
    sphere.position.y = m.wormhole.pos[1];
    scene.add(sphere);
    wormholes.push(sphere);
  }
  if (m.boss) {
    if (!boss) {
      let obj = new THREE.Object3D();
      let sphere = new THREE.Mesh(wormholegeometry, bossmaterial);
      let health = new THREE.Mesh(bosshealthgeometry, bosshealthmaterial);

      health.position.y += 70;
      obj.add(sphere);
      obj.add(health);
      scene.add(obj);
      boss = obj;
    }
    boss.children[1].scale.x = m.boss.health / 255;
    boss.position.x = m.boss.pos[0];
    boss.position.y = m.boss.pos[1];
  } else if (boss) {
    boss.remove(boss.children[1]);
    boss.remove(boss.children[0]);
    scene.remove(boss);
    boss = undefined;
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
          let ring = new THREE.RingBufferGeometry(68, 80, 10, 1, Math.PI, Math.PI / 2);
          let mesh = new THREE.Mesh(ring, healthMaterial);
          mesh.position.set(0, 0, 0);
          container.add(mesh);
        }
        {
          let ring = new THREE.RingBufferGeometry(68, 80, 10, 1, 1.5 * Math.PI, Math.PI / 2);
          let mesh = new THREE.Mesh(ring, manaMaterial);
          mesh.position.set(0, 0, 0);
          container.add(mesh);
        }
        {
          let ring = new THREE.RingBufferGeometry(80, 92, 10, 1, (4 / 3) * Math.PI, Math.PI / 3);
          let mesh = new THREE.Mesh(ring, cooldownMaterial);
          mesh.position.set(0, 0, 0);
          container.add(mesh);
        }
        if (p.name) {
          let text = createText(p.name, 30);
          uiElements.push(text);
          uiMaterials.push(undefined);
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
        let ring = new THREE.RingBufferGeometry(68, 80, 10, 1, Math.PI * (1.5 - p.health / 510), (Math.PI * p.health) / 510);
        (sprites[p.id].children[1] as THREE.Mesh).geometry.dispose();
        (sprites[p.id].children[1] as THREE.Mesh).geometry = ring;
      }
      {
        let ring = new THREE.RingBufferGeometry(68, 80, 10, 1, 1.5 * Math.PI, (Math.PI * p.mana) / 510);
        (sprites[p.id].children[2] as THREE.Mesh).geometry.dispose();
        (sprites[p.id].children[2] as THREE.Mesh).geometry = ring;
      }
      {
        let cooldown = p.class == "Quickshot" ? 750 : 1000;
        let timeDiff = Math.min(p.shot_time, cooldown);
        if (timeDiff == 0) {
          sprites[p.id].children[3].visible = false;
        } else {
          sprites[p.id].children[3].visible = true;
          let ring = new THREE.RingBufferGeometry(80, 92, 10, 1, (4 / 3) * Math.PI, ((Math.PI / 3) * timeDiff) / cooldown);
          (sprites[p.id].children[3] as THREE.Mesh).geometry.dispose();
          (sprites[p.id].children[3] as THREE.Mesh).geometry = ring;
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

  darkenUI();
  bloomComposer.render();
  restoreUI();

  finalComposer.render();
};

window.addEventListener("resize", () => {
  aspect = window.innerHeight / window.innerWidth;
  camera.top = view * aspect;
  camera.bottom = -view * aspect;
  camera.updateProjectionMatrix();
  renderer.setSize(window.innerWidth, window.innerHeight);
});
