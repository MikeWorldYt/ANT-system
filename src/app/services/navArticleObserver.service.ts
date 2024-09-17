import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class ArticleService {
  private observer: IntersectionObserver;
  private currentArticle$: BehaviorSubject<string> = new BehaviorSubject<string>('');

  constructor() {
    this.observer = new IntersectionObserver(this.intersectionCallback.bind(this), {
      // threshold: 0.1, // 10% viewport
      rootMargin: '-32px 0px -90% 0px' // Ajuste para 2rem desde el top (-32px)
    });
  }

  private intersectionCallback(entries: IntersectionObserverEntry[]) {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        this.currentArticle$.next(entry.target.id);
      }
    });
  }

  observe(element: Element) {
    this.observer.observe(element);
  }

  unobserve(element: Element) {
    this.observer.unobserve(element);
  }

  getcurrentArticle(): Observable<string> {
    return this.currentArticle$.asObservable();
  }
}

// This server get the <section name> when scrolling
// and it intersect in an imaginary line