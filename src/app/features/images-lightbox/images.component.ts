import { CommonModule } from '@angular/common';
import { Component, Input } from '@angular/core';
import { LanguageService } from '../../services/navLanguage.service';
import { Subscription } from 'rxjs';
import { content } from '../../docs/content/content';
import { LightboxComponent } from '../lightbox/lightbox.component';

@Component({
  selector: 'images',
  standalone: true,
  imports: [
    CommonModule,
    LightboxComponent,
  ],
  templateUrl: './images.component.html',
  styleUrl: './images.component.css'
})
export class ImagesComponent {
  @Input() path!: [string, string, string];
  @Input() article!: string;
  @Input() write!: string;

  constructor(
    private languageService: LanguageService,
  ) { }

  imageSrc: string = '';

  ngOnInit(): void {
    this.article = this.concatArticle(this.article);
    this.subscribeToLanguageChanges();
    this.resolveContent();
  }

  languageSubscription!: Subscription;
  private subscribeToLanguageChanges(): void {
    this.languageSubscription = this.languageService.currentLanguage$.subscribe((Lang) => {
      this.path[0] = Lang;
      this.resolveContent();
    });
  }

  private resolveContent(): void {
    const [lang, title, page] = this.path;
    const article = this.article;
    const write = this.write;
    this.imageSrc = content[lang][title][page][article][write];
  }




  private concatArticle(article: string): string {
    return `article_${article}`;
  }


}
