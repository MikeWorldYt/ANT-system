import { Injectable } from '@angular/core';

@Injectable({
  providedIn: 'root'
})
export class IntersectionService {
  private observer: IntersectionObserver;
  private currentId: string = '';

  constructor() {
    this.observer = new IntersectionObserver(this.intersectionCallback.bind(this), {
      threshold: 0.1 // 10% viewport
    });
  }

  private intersectionCallback(entries: IntersectionObserverEntry[]) {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        this.currentId = entry.target.id;
        console.log('Sección visible:', this.currentId);
      }
    });
  }

  observe(element: Element) {
    this.observer.observe(element);
  }

  unobserve(element: Element) {
    this.observer.unobserve(element);
  }

  getCurrentId(): string {
    return this.currentId;
  }
}