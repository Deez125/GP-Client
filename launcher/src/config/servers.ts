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
  /** Server address for quick-join (host or host:port; uses MC's SRV resolver). */
  address: string;
  /**
   * Explicit host:port used for the live status ping. Set this when the server
   * isn't on the default port / SRV resolution is unreliable, so we ping the
   * exact endpoint instead of guessing. Falls back to `address` if unset.
   */
  statusAddress?: string;
}

export const SERVERS: ServerDef[] = [
  {
    id: "server1",
    name: "Epstein's Island",
    icon: icon1,
    hero: hero1,
    address: "play.gayporn.tech",
    statusAddress: "72.61.74.46:25565",
  },
  {
    id: "server2",
    name: "Epstein's Skyblock",
    icon: icon2,
    hero: hero2,
    address: "skyblock.gayporn.tech",
    statusAddress: "72.61.74.46:25566",
  },
];
