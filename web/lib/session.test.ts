import { describe, it, expect } from "vitest";

import { ORIGINE_LOCALE, origineDeLaRequete } from "./autorisation";
import {
  depotDeSession,
  etapesDeDeconnexion,
  memeDepotDeSession,
  suiteDeLaConnexion,
} from "./session";

// La session d'un lancement à l'autre, et la sortie (issue #64, FR-014,
// FR-015). Ce qui se vit en quittant puis en rouvrant l'application reste sur
// la liste de vérifications manuelles ; ce qui se décide, lui, se vérifie ici.

describe("le stockage de la session (FR-014, FR-070)", () => {
  it("une session écrite sous l'origine locale fixe se relit sous cette même origine", () => {
    expect(memeDepotDeSession(ORIGINE_LOCALE, ORIGINE_LOCALE)).toBe(true);
  });

  it("une autre origine ne la trouve pas", () => {
    // `localhost` et `127.0.0.1` désignent la même machine et ne partagent
    // aucun cookie. C'est la raison pour laquelle l'échangeur d'autorisation
    // se donne tant de mal à rester sur l'origine que le client a demandée :
    // une session posée sur `localhost` serait invisible à la fenêtre, qui
    // n'admet que l'origine locale fixe.
    expect(memeDepotDeSession("http://localhost:51789", ORIGINE_LOCALE)).toBe(false);
    expect(memeDepotDeSession("https://vibemap.example", ORIGINE_LOCALE)).toBe(false);
  });

  it("l'échangeur pose la session sous l'origine que la fenêtre rechargera", () => {
    // Next fabrique `request.url` à partir de son propre nom d'hôte et rend
    // `localhost` même servi sur `127.0.0.1`. Si l'échangeur suivait cette
    // proposition, la session serait écrite dans un autre dépôt de cookies et
    // le lancement suivant redemanderait de se connecter - exactement ce que
    // FR-014 interdit.
    const pose = origineDeLaRequete(
      "http://localhost:51789/auth/callback?code=abc",
      "127.0.0.1:51789",
    );

    expect(pose).toBe(ORIGINE_LOCALE);
    expect(memeDepotDeSession(pose, ORIGINE_LOCALE)).toBe(true);
  });

  it("une origine illisible ne se range nulle part, et ne coïncide avec rien", () => {
    expect(depotDeSession("pas une adresse")).toBe(null);
    expect(memeDepotDeSession("pas une adresse", "pas une adresse")).toBe(false);
  });

  it("le port ne sépare pas deux dépôts de session : c'est l'hôte qui le fait", () => {
    // Ce test dit une chose contre-intuitive, et c'est pour cela qu'il existe :
    // un cookie n'a jamais eu de port dans sa portée. Ce que le port fixe tient
    // est ailleurs - l'adresse de retour déclarée chez le fournisseur
    // d'identité, et une fenêtre qui recharge la MÊME origine à chaque
    // lancement. Qui viendrait « corriger » ce test en croyant durcir FR-014
    // durcirait une règle que le navigateur n'applique pas.
    expect(memeDepotDeSession("http://127.0.0.1:3000", ORIGINE_LOCALE)).toBe(true);
  });
});

describe("se déconnecter (FR-015)", () => {
  it("dans l'application, la déconnexion arrête le lecteur", () => {
    // La seconde moitié de FR-015, celle qui ne se voit pas à l'écran : un
    // lecteur laissé en marche tiendrait encore le poste et continuerait
    // d'émettre au nom d'une session fermée.
    expect(etapesDeDeconnexion(true)).toEqual({ arreterLeLecteur: true });
  });

  it("hors de l'application, il n'y a aucun lecteur à arrêter", () => {
    expect(etapesDeDeconnexion(false)).toEqual({ arreterLeLecteur: false });
  });
});

describe("se reconnecter sans quitter l'application (issue #64)", () => {
  it("la session rouverte remet le lecteur en marche et ramène la fenêtre devant", () => {
    expect(suiteDeLaConnexion(true)).toEqual({
      relancerLeLecteur: true,
      revenirAuPremierPlan: true,
    });
  });

  it("dans un navigateur ordinaire, il n'y a ni lecteur ni fenêtre à ramener", () => {
    expect(suiteDeLaConnexion(false)).toEqual({
      relancerLeLecteur: false,
      revenirAuPremierPlan: false,
    });
  });
});
