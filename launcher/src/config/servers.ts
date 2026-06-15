// The server list (mock for now). Each server has a sidebar icon image and a
// hero background image shown when it's selected. Edit names/images here.
import icon1 from "../assets/icon.png";
import icon2 from "../assets/icon2.png";
import hero1 from "../assets/pic.png";
import hero2 from "../assets/pic2.png";

export interface ServerDef {
  id: string;
  name: string;
  /** Sidebar icon image. */
  icon: string;
  /** Hero background image shown when this server is selected. */
  hero: string;
}

export const SERVERS: ServerDef[] = [
  { id: "server1", name: "Epstein's Island", icon: icon1, hero: hero1 },
  { id: "server2", name: "Epstein's Skyblock", icon: icon2, hero: hero2 },
];
