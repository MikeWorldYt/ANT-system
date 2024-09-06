import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class IntersectionService {
  private observer: IntersectionObserver;
  private currentId: BehaviorSubject<string> = new BehaviorSubject<string>('');

  constructor() {
    this.observer = new IntersectionObserver(this.intersectionCallback.bind(this), {
      // threshold: 0.1, // 10% viewport
      rootMargin: '-32px 0px -90% 0px' // Ajuste para 2rem desde el top (-32px)
    });
  }

  private intersectionCallback(entries: IntersectionObserverEntry[]) {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        this.currentId.next(entry.target.id);
      }
    });
  }

  observe(element: Element) {
    this.observer.observe(element);
  }

  unobserve(element: Element) {
    this.observer.unobserve(element);
  }

  getCurrentId(): Observable<string> {
    return this.currentId.asObservable();
  }
}