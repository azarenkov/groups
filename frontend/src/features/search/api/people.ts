import { http } from "../../../shared/api/http";

export type PersonRelation = "none" | "friend" | "outgoing" | "incoming";

export interface PersonHit {
  id: string;
  name: string;
  relation: PersonRelation;
  acceptsFriendRequests: boolean;
}

export const peopleAPI = {
  async search(query: string, limit = 8): Promise<PersonHit[]> {
    const response = await http.get("/v1/search/people", { params: { q: query, limit } });
    return response.data.data.users;
  },
};
